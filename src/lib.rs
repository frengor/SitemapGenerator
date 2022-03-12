#![forbid(unsafe_code)]

use std::sync::Arc;

use tokio::sync::{mpsc, Semaphore};
use url::Url;

use crate::site_analyzer::types::{StartTaskInfo, Response};
pub use crate::options::*;
use crate::site_analyzer::types::StartTaskInfo;
pub use crate::site_analyzer::types::Validator;

pub mod utils;
pub(crate) mod options;

pub(crate) mod site_analyzer {
    pub mod processing;
    pub mod types;
}

pub async fn analyze(sites_to_analyze: impl Iterator<Item=Url>, validator: Validator, options: Options) -> impl IntoIterator<Item=Arc<Url>> {
    let max_task_count = options.max_task_count();
    let max_recursion = options.max_recursion();
    let (tx, mut rx) = mpsc::channel(max_task_count);

    let mut sites = std::collections::HashSet::new();
    let options = Arc::new(options);
    let sem = Arc::new(Semaphore::new(max_task_count));

    {
        let iter = sites_to_analyze
        .filter(|site| validator.is_valid(site))
        .map(Arc::new)
        .filter(|site| sites.insert(Arc::clone(site)));

        for site in iter {
            StartTaskInfo {
                site,
                tx: tx.clone(),
                validator: validator.clone(),
                recursion: max_recursion,
            }.spawn_task(sem.clone(), options.clone()).await;
        }
    }

    // Drop our sender
    drop(tx);

    // Main loop
    while let Some(start_task_info) = rx.recv().await {
        if validator.is_valid(&start_task_info.site) && sites.insert(start_task_info.site.clone()) {
            start_task_info.spawn_task(sem.clone(), options.clone()).await;
        }
    }

    sites
}
