#![forbid(unsafe_code)]

use std::cell::Cell;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use futures::{stream, StreamExt};
use reqwest::{Client, StatusCode};
use reqwest::redirect::{Attempt, Policy};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::Semaphore;
use url::Url;

pub use crate::options::*;
use crate::site_analyzer::types::TaskInfo;
pub use crate::site_analyzer::types::Validator;
use crate::utils::println;

pub mod utils;
pub(crate) mod options;

pub(crate) mod site_analyzer {
    pub mod processing;
    pub mod types;
}

const APP_USER_AGENT: &str = concat!(
env!("CARGO_PKG_NAME"),
"/",
env!("CARGO_PKG_VERSION"),
);

pub async fn analyze(sites_to_analyze: impl Iterator<Item=Url>, validator: Validator, mut options: Options) -> impl IntoIterator<Item=Arc<Url>> {
    let max_task_count = options.max_task_count();
    let max_recursion = options.max_recursion();
    let verbose = options.verbose();
    let (tx, mut rx): (UnboundedSender<TaskInfo>, UnboundedReceiver<TaskInfo>) = mpsc::unbounded_channel();

    // The first Arc and the Mutex are necessary to make sites movable between threads
    // The Cell is used at the end of this function to allow the HashSet to be returned
    let sites = Sites::new();
    let options = Arc::new(options);
    let sem = Arc::new(Semaphore::new(max_task_count));

    let client = create_client(sites.clone(), validator.clone(), verbose);

    {
        stream::iter(sites_to_analyze)
        .map(Arc::new)
        .filter(|site| std::future::ready(validator.is_valid(site)))
        .filter(|site| {
            let site = site.clone();
            async {
                sites.access_map(|hashset| hashset.insert(site))
            }
        })
        .for_each(|site| {
            TaskInfo {
                site,
                tx: tx.clone(),
                validator: validator.clone(),
                recursion: max_recursion,
            }.spawn_task(client.clone(), sem.clone(), options.clone())
        })
        .await;
    }

    // Drop our sender
    drop(tx);

    while let Some(task_info) = rx.recv().await {
        if /*validator.is_valid(&task_info.site) check already done &&*/ sites.access_map(|hashset| hashset.insert(task_info.site.clone())) {
            task_info.spawn_task(client.clone(), sem.clone(), options.clone()).await;
        }
    }

    /*let (oneshot_tx, oneshot_rx) = oneshot::channel();
    {
        let sites = sites.clone();
        let options = options.clone();
        let client = client.clone();
        let sem = sem.clone();
        thread::Builder::new().name("SitemapGenerator - Main thread".to_string()).spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async move {
                // Main loop
                while let Some(task_info) = rx.recv().await {
                    if /*validator.is_valid(&task_info.site) check already done &&*/ sites.lock().unwrap().get_mut().insert(task_info.site.clone()) {
                        task_info.spawn_task(client.clone(), sem.clone(), options.clone()).await;
                    }
                }

                let _ = oneshot_tx.send(sites);
            });
        });
    }*/

    drop(client);

    let make_it_compile = sites.inner.lock().unwrap().replace(HashSet::with_capacity(0));
    make_it_compile
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Sites {
    inner: Arc<Mutex<Cell<HashSet<Arc<Url>>>>>,
}

impl Sites {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Sites {
        Sites {
            inner: Arc::new(Mutex::new(Cell::new(std::collections::HashSet::new()))),
        }
    }

    pub fn access_map<F, R>(&self, f: F) -> R
        where
        F: FnOnce(&mut HashSet<Arc<Url>>) -> R,
        R: Sized
    {
        f(self.inner.lock().unwrap().get_mut())
    }
}

fn create_client(sites: Sites, validator: Validator, verbose: bool) -> Client {
    reqwest::Client::builder()
    .user_agent(APP_USER_AGENT)
    .redirect(Policy::custom(move |attempt| {
        fn check_validated(validator: &Validator, attempt: &Attempt) -> Option<anyhow::Error> {
            if !validator.is_valid(attempt.url()) {
                let previous = match attempt.previous().last() {
                    Some(prev) => {
                        prev.as_str()
                    },
                    None => "None",
                };
                Some(anyhow!(r#""{}" is permanently moved to "{}", which is not on to analyze"#, previous, attempt.url().as_str()))
            } else {
                None
            }
        }

        if attempt.status() == StatusCode::MOVED_PERMANENTLY {
            sites.access_map(|hashset| hashset.remove(attempt.url()));

            if verbose {
                tokio::spawn(println(format!("Removed {}", attempt.url().as_str())));
            }

            if let Some(error) = check_validated(&validator, &attempt) {
                return attempt.error(error);
            }
            Policy::limited(10).redirect(attempt)
        } else {
            if let Some(error) = check_validated(&validator, &attempt) {
                return attempt.error(error);
            }

            let arc = Arc::new(attempt.url().clone());
            let insert_result = if verbose {
                if sites.access_map(|hashset| hashset.insert(arc.clone())) {
                    #[repr(transparent)]
                    struct AsRefImpl {
                        arc: Arc<Url>,
                    }
                    impl AsRef<str> for AsRefImpl {
                        #[inline(always)]
                        fn as_ref(&self) -> &str {
                            self.arc.as_str()
                        }
                    }
                    crate::utils::verbose(AsRefImpl { arc });
                    true
                } else {
                    false
                }
            } else {
                sites.access_map(|hashset| hashset.insert(arc))
            };
            if insert_result {
                Policy::limited(10).redirect(attempt)
            } else {
                attempt.stop()
            }
        }
    }))
    .build().unwrap()
}
