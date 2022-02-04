#![allow(non_snake_case)]

use std::collections::HashSet;
use std::sync::Arc;

use hyper_tls::HttpsConnector;
use tokio::sync::{mpsc, oneshot, Semaphore};

pub use crate::processing::process;
pub use crate::types::*;
pub use crate::utils::*;

mod utils;
mod processing;
mod types;

const CONCURRENT_TASKS: usize = 64;

static SEM: Semaphore = Semaphore::const_new(CONCURRENT_TASKS);

#[tokio::main]
async fn main() {
    let client = Client::builder().build(HttpsConnector::new());
    let (tx, mut rx) = mpsc::channel(CONCURRENT_TASKS);

    let sites_to_analyze = vec![
        "185.25.204.194",
        "https://docs.rs/hyper/0.14.16/hyper/client/struct.Client.html",
        "https://frengor.com/",
        "https://frengor.com/UltimateAdvancementAPI/",
    ];

    // Make sure we don't deadlock!
    if SEM.available_permits() < sites_to_analyze.len() {
        SEM.add_permits(sites_to_analyze.len());
    }

    let mut sites = HashSet::new();

    for site in sites_to_analyze {
        let site = Arc::new(site.to_string());
        if is_valid_site(&site) && sites.insert(Arc::clone(&site)) {
            /*let permit = SEM.acquire().await?;
            // The call to tokio::spawn avoids deadlock
            tokio::spawn(async move {
                print_err(spawn_task(StartTaskInfo { site, tx, client }, permit).await).await;
            });*/
            StartTaskInfo { site, tx: tx.clone(), client: client.clone() }.spawn_task(&SEM).await;
        }
    }

    // Drop our sender
    drop(tx);

    let (close_tx, close_rx) = oneshot::channel();

    tokio::spawn(async move {
        while let Some(site_info) = rx.recv().await {
            let response = if is_valid_site(&site_info.site) && sites.insert(Arc::clone(&site_info.site)) {

                // Site is new and to analyze
                Response { to_process: true }
            } else {
                Response { to_process: false }
            };
            let _ = site_info.responder.send(response);
        }
        let _ = close_tx.send(sites);
    });

    for site in close_rx.await.expect("Cannot get found sites.") {
        println!("{}", site);
    }
}

pub fn is_valid_site(site: &str) -> bool {
    // TODO: write proper function
    site.contains("frengor.com")
}
