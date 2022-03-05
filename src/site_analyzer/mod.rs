use std::collections::HashSet;
use std::sync::Arc;

use hyper_tls::HttpsConnector;
use tokio::sync::{mpsc, Semaphore};
use url::Url;

use types::*;

pub mod processing;
pub mod types;

static SEM: Semaphore = Semaphore::const_new(0);

pub async fn analyze(sites_to_analyze: impl Iterator<Item=Url>, validator: Validator, max_task_count: usize) -> HashSet<impl AsRef<Url>> {
    SEM.add_permits(max_task_count);

    let client = Client::builder().build(HttpsConnector::new());
    let (tx, mut rx) = mpsc::channel(max_task_count);

    let mut sites = HashSet::new();

    let iter = sites_to_analyze
    .filter(|site| validator.is_valid(site))
    .map(Arc::new)
    .filter(|site| sites.insert(Arc::clone(site)));

    for site in iter {
        StartTaskInfo { site, tx: tx.clone(), client: client.clone(), validator: validator.clone() }.spawn_task(&SEM).await;
    }

    // Drop our sender
    drop(tx);

    while let Some(site_info) = rx.recv().await {
        let response = if validator.is_valid(&site_info.site) && sites.insert(Arc::clone(&site_info.site)) {

            // Site is new and to analyze
            Response { to_process: true }
        } else {
            Response { to_process: false }
        };
        let _ = site_info.responder.send(response);
    }

    sites
}
