use std::sync::Arc;

use tokio::sync::{mpsc, Semaphore};
use url::Url;

use crate::site_analyzer::types::{StartTaskInfo, Response};
pub use crate::site_analyzer::types::Validator;
pub use crate::options::*;

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

    {
        let iter = sites_to_analyze
        .filter(|site| validator.is_valid(site))
        .map(Arc::new)
        .filter(|site| sites.insert(Arc::clone(site)));

        let options = Arc::new(options);
        let sem = Arc::new(Semaphore::new(max_task_count));

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
    while let Some(site_info) = rx.recv().await {
        let _ = site_info.responder.send(Response {
            to_process: validator.is_valid(&site_info.site) && sites.insert(Arc::clone(&site_info.site)),
        });
    }

    sites
}
