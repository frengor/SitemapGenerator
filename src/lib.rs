#![forbid(unsafe_code)]

use std::future;
use std::sync::{Arc, Mutex};

use futures::{stream, StreamExt};
use reqwest::redirect::Policy;
use tokio::sync::{mpsc, Semaphore};
use url::Url;

pub use crate::options::*;
use crate::site_analyzer::types::StartTaskInfo;
pub use crate::site_analyzer::types::Validator;

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
    let (tx, mut rx) = mpsc::channel(max_task_count);

    let sites = Mutex::new(std::collections::HashSet::new());
    let options = Arc::new(options);
    let sem = Arc::new(Semaphore::new(max_task_count));

    let client = reqwest::Client::builder()
    .user_agent(APP_USER_AGENT)
    .redirect(Policy::none())
    .build().unwrap();

    {
        stream::iter(sites_to_analyze)
        .map(Arc::new)
        .filter(|site| future::ready(validator.is_valid(site)))
        .filter(|site| {
            let site = site.clone();
            async {
                sites.lock().unwrap().insert(site)
            }
        })
        .for_each(|site| {
            StartTaskInfo {
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
    while let Some(start_task_info) = rx.recv().await {
        if validator.is_valid(&start_task_info.site) && sites.lock().unwrap().insert(start_task_info.site.clone()) {
            start_task_info.spawn_task(client.clone(), sem.clone(), options.clone()).await;
        }
    }

    sites.into_inner().unwrap()
}
