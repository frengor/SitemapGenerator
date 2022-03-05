use std::collections::HashSet;
use std::sync::Arc;

use hyper_tls::HttpsConnector;
use tokio::sync::{mpsc, Semaphore};
use tokio::sync::OnceCell;
use url::Url;

use types::*;

pub mod processing;
pub mod types;

static SEM: Semaphore = Semaphore::const_new(0);
static OPTIONS: OnceCell<AnalyzerOptions> = OnceCell::const_new();

pub struct AnalyzerOptions {
    max_task_count: usize,
    remove_query_and_fragment: bool,
}

impl AnalyzerOptions {
    #[inline]
    pub fn new(max_task_count: usize, remove_query_and_fragment: bool) -> AnalyzerOptions {
        AnalyzerOptions {
            max_task_count,
            remove_query_and_fragment,
        }
    }

    #[inline]
    pub fn max_task_count(&self) -> usize {
        self.max_task_count
    }

    #[inline]
    pub fn remove_query_and_fragment(&self) -> bool {
        self.remove_query_and_fragment
    }
}

pub async fn analyze(sites_to_analyze: impl Iterator<Item=Url>, validator: Validator, analyzerOptions: AnalyzerOptions) -> HashSet<impl AsRef<Url>> {
    let max_task_count = analyzerOptions.max_task_count();
    if OPTIONS.set(analyzerOptions).is_err() {
        panic!("Cannot set OPTIONS.");
    }
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
        let _ = site_info.responder.send(Response {
            to_process: validator.is_valid(&site_info.site) && sites.insert(Arc::clone(&site_info.site)),
        });
    }

    sites
}
