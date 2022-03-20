#![forbid(unsafe_code)]

use std::cell::Cell;
use std::collections::HashSet;
use std::future;
use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use futures::{stream, StreamExt};
use reqwest::redirect::Policy;
use reqwest::StatusCode;
use tokio::sync::{mpsc, Semaphore};
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

pub async fn analyze(sites_to_analyze: impl Iterator<Item=Url>, validator: Validator, options: Options) -> impl IntoIterator<Item=Arc<Url>> {
    let max_task_count = options.max_task_count();
    let max_recursion = options.max_recursion();
    let verbose = options.verbose();
    let (tx, mut rx) = mpsc::channel(max_task_count);

    // The first Arc and the Mutex are necessary to make sites movable between threads
    // The Cell is used at the end of this function to allow the HashSet to be returned
    let sites = Arc::new(Mutex::new(Cell::new(std::collections::HashSet::new())));
    let options = Arc::new(options);
    let sem = Arc::new(Semaphore::new(max_task_count));

    // TODO: move client building to another function
    let sites_cloned = sites.clone();
    let validator_cloned = validator.clone();
    let client = reqwest::Client::builder()
    .user_agent(APP_USER_AGENT)
    .redirect(Policy::custom(move |attempt| {
        let mut mutex = sites_cloned.lock().unwrap();
        let hashset = mutex.get_mut();

        let check_validated = || -> Option<anyhow::Error> {
            if !validator_cloned.is_valid(attempt.url()) {
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
        };

        if attempt.status() == StatusCode::MOVED_PERMANENTLY {
            hashset.remove(attempt.url());

            if let Some(error) = check_validated() {
                return attempt.error(error);
            }
            if verbose {
                tokio::spawn(println(format!("Removed {}", attempt.url().as_str())));
            }
            Policy::limited(10).redirect(attempt)
        } else {
            if let Some(error) = check_validated() {
                return attempt.error(error);
            }

            let arc = Arc::new(attempt.url().clone());
            let insert_result = if verbose {
                if hashset.insert(arc.clone()) {
                    #[repr(transparent)]
                    struct AsRefImpl {
                        arc: Arc<Url>,
                    }
                    impl AsRef<str> for AsRefImpl {
                        #[inline]
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
                hashset.insert(arc)
            };
            if insert_result {
                Policy::limited(10).redirect(attempt)
            } else {
                attempt.stop()
            }
        }
    }))
    .build().unwrap();

    {
        stream::iter(sites_to_analyze)
        .map(Arc::new)
        .filter(|site| future::ready(validator.is_valid(site)))
        .filter(|site| {
            let site = site.clone();
            async {
                sites.lock().unwrap().get_mut().insert(site)
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

    // Main loop
    while let Some(task_info) = rx.recv().await {
        if /*validator.is_valid(&task_info.site) check already done &&*/ sites.lock().unwrap().get_mut().insert(task_info.site.clone()) {
            task_info.spawn_task(client.clone(), sem.clone(), options.clone()).await;
        }
    }

    // Drop the client
    drop(client);

    let make_it_compile = sites.lock().unwrap().replace(HashSet::with_capacity(0));
    make_it_compile
}
