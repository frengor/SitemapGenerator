#![allow(non_snake_case)]

use std::collections::HashSet;
use std::sync::Arc;

use anyhow::{anyhow, bail, Result};
use futures::{FutureExt, stream, StreamExt};
use futures::future::BoxFuture;
use hyper_tls::HttpsConnector;
use tokio::io::{AsyncWriteExt, stderr};
use tokio::sync::{mpsc, oneshot, Semaphore, SemaphorePermit};

pub use crate::processing::process;
pub use crate::types::*;
pub use crate::utils::*;

mod utils;
mod processing;
mod types;

const CONCURRENT_TASKS: usize = 64;

static SEM: Semaphore = Semaphore::const_new(CONCURRENT_TASKS);

#[tokio::main]
async fn main() -> Result<()> {
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
            let client = client.clone();
            let tx = tx.clone();
            let permit = SEM.acquire().await?;
            // The call to tokio::spawn avoids deadlock
            tokio::spawn(async move {
                print_err(spawn_task(StartTaskInfo { site, tx, client }, permit).await).await;
            });
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

    for site in close_rx.await? {
        println!("{}", site);
    }
    Ok(())
}

fn is_valid_site(site: &str) -> bool {
    // TODO: write proper function
    site.contains("frengor.com")
}

// https://rust-lang.github.io/async-book/07_workarounds/04_recursion.html
fn spawn_task<T: ClientBounds>(task_info: StartTaskInfo<T>, permit: SemaphorePermit<'static>) -> BoxFuture<'static, Result<()>> {
    async move {
        let links = match processing::analyze_html(&task_info).await {
            Ok(links) => links,
            Err(err) => bail!(r#"An error occurred analyzing "{}": {err}"#, &task_info.site),
        };

        // Release semaphore before acquiring again
        drop(permit);

        stream::iter(links)
        .filter(|link| {
            let ret = is_valid_site(&link);
            async move { ret }
        })
        .filter_map(|link| async {
            let site = Arc::new(link);
            let (o_tx, o_rx) = oneshot::channel();
            if let Err(_) = task_info.tx.send(SiteInfo { site: Arc::clone(&site), responder: o_tx }).await {
                eprintln("Couldn't send site to main task!", &site).await;
                return None;
            }

            match o_rx.await {
                Ok(resp) if resp.to_process() => {
                    Some(StartTaskInfo {
                        site,
                        tx: task_info.tx.clone(),
                        client: task_info.client.clone(),
                    })
                },
                Ok(_) => None,
                Err(_) => {
                    eprintln("Couldn't receive the response from main task!", &site).await;
                    None
                }
            }
        })
        .for_each(|task_info| async {
            if let Ok(permit) = SEM.acquire().await {
                tokio::spawn(async move {
                    print_err(spawn_task(task_info, permit).await).await;
                });
            } else {
                eprintln("Cannot acquire SEM", &task_info.site).await;
            }
        })
        .await;
        Ok(())
    }.boxed()
}

async fn print_err<R>(res: Result<R>) {
    if let Err(error) = res {
        let _ = stderr().write(format!("{error}\n").as_bytes()).await;
    }
}
