use std::collections::HashSet;
use std::sync::Arc;

use tokio::sync::{mpsc, Semaphore};
use url::Url;

use types::*;
use crate::get_options;

pub mod processing;
pub mod types;

static SEM: Semaphore = Semaphore::const_new(0);

pub async fn analyze(sites_to_analyze: impl Iterator<Item=Url>, validator: Validator) -> HashSet<impl AsRef<Url>> {
    let options = get_options();

    SEM.add_permits(options.max_task_count());

    let (tx, mut rx) = mpsc::channel(options.max_task_count());

    let mut sites = HashSet::new();

    let iter = sites_to_analyze
    .filter(|site| validator.is_valid(site))
    .map(Arc::new)
    .filter(|site| sites.insert(Arc::clone(site)));

    for site in iter {
        StartTaskInfo {
            site,
            tx: tx.clone(),
            validator: validator.clone(),
            recursion: options.max_recursion(),
        }.spawn_task(&SEM).await;
    }

    // Drop our sender
    drop(tx);

    while let Some(site_info) = rx.recv().await {
        let _ = site_info.responder.send(Response {
            to_process: validator.is_valid(&site_info.site) && sites.insert(Arc::clone(&site_info.site)),
        });
    }

    sites
}
