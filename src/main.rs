#![allow(non_snake_case)]

use std::collections::HashSet;
use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use anyhow::Result;
use futures::{FutureExt, stream, StreamExt};
use futures::future::BoxFuture;
use hyper::Body;
use hyper_tls::HttpsConnector;
use scraper::Selector;
use tokio::sync::{mpsc, oneshot, Semaphore, SemaphorePermit};
use tokio::sync::mpsc::Sender;

pub use crate::processing::process;
pub use crate::utils::*;

mod utils;
mod processing;

pub type Client<T> = hyper::Client<T, Body>;

const CONCURRENT_TASKS: usize = 20;

static SEM: Semaphore = Semaphore::const_new(CONCURRENT_TASKS);

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder().build(HttpsConnector::new());
    let (tx, mut rx) = mpsc::channel(64);

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
                spawn_task(StartTaskInfo { site, tx, client }, permit).await;
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
            site_info.responder.send(response).unwrap();
        }
        let _ = close_tx.send(sites);
    });

    for site in close_rx.await? {
        println!("{}", site);
    }
    Ok(())
}

struct StartTaskInfo<T: ClientBounds> {
    site: Arc<String>,
    tx: Sender<SiteInfo>,
    client: Client<T>,
}

struct SiteInfo {
    site: Arc<String>,
    responder: oneshot::Sender<Response>,
}

impl Debug for SiteInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.site)
    }
}

#[derive(Clone, Debug)]
struct Response {
    to_process: bool,
}

impl Response {
    fn to_process(&self) -> bool {
        self.to_process
    }
}

fn is_valid_site(site: &str) -> bool {
    // TODO: write proper function
    site.contains("frengor.com")
}

// https://rust-lang.github.io/async-book/07_workarounds/04_recursion.html
fn spawn_task<T: ClientBounds>(task_info: StartTaskInfo<T>, permit: SemaphorePermit<'static>) -> BoxFuture<'static, ()> {
    async move {
        let links: Option<Vec<_>> = {
            let html_res = process(&task_info.site, task_info.client.clone()).await;
            match html_res {
                Ok(html) => match Selector::parse("a") {
                    Ok(selector) => {
                        Some(html.select(&selector)
                        .filter_map(|a_elem| {
                            if let Some(link) = a_elem.value().attr("href") {
                                // TODO: properly handle relative links like "/discord" inside a tags
                                Some(link.to_string())
                            } else {
                                None
                            }
                        })
                        .collect())
                    },
                    Err(e) => {
                        eprintln!("An error occurred parsing selector: {:?}", e.kind);
                        None
                    },
                },
                Err(e) => {
                    eprintln!(r#"An error occurred analyzing "{}": {e}"#, task_info.site);
                    None
                },
            }
        };

        if let Some(links) = links {
            stream::iter(links)
            .filter_map(|link| async {
                let site = Arc::new(link);
                let (o_tx, o_rx) = oneshot::channel();
                task_info.tx.send(SiteInfo { site: Arc::clone(&site), responder: o_tx }).await.unwrap();
                if o_rx.await.unwrap().to_process() {
                    Some(StartTaskInfo {
                        site,
                        tx: task_info.tx.clone(),
                        client: task_info.client.clone(),
                    })
                } else {
                    None
                }
            })
            .for_each(|task_info| async {
                let permit = SEM.acquire().await.unwrap();
                tokio::spawn(async move {
                    spawn_task(task_info, permit).await;
                });
            })
            .await;
        }
        // Explicitly release permit
        drop(permit);
    }.boxed()
}
