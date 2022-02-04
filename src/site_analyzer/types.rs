use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use async_recursion::async_recursion;
use futures::{stream, StreamExt};
use hyper::Body;
use tokio::sync::{oneshot, Semaphore};
use tokio::sync::mpsc::Sender;

use crate::utils::*;

use super::processing::{self, *};

pub type Client<T> = hyper::Client<T, Body>;

pub struct StartTaskInfo<T: ClientBounds> {
    pub site: Arc<String>,
    pub tx: Sender<SiteInfo>,
    pub client: Client<T>,
}

impl<T: ClientBounds> StartTaskInfo<T> {
    #[async_recursion]
    pub async fn spawn_task(self, semaphore: &'static Semaphore) {
        let permit = match semaphore.acquire().await {
            Ok(permit) => permit,
            Err(_) => {
                eprintln("cannot spawn task", &self.site).await;
                return;
            }
        };

        tokio::spawn(async move {
            let links = match processing::analyze_html(&self).await {
                Ok(links) => links,
                Err(err) => {
                    eprintln(err, &self.site).await;
                    return;
                },
            };

            let tmp: Vec<_> = stream::iter(links)
            .filter(|link| {
                let ret = is_valid_site(link);
                async move { ret }
            })
            .filter_map(|link| async {
                let site = Arc::new(link);
                let (o_tx, o_rx) = oneshot::channel();
                if self.tx.send(SiteInfo { site: Arc::clone(&site), responder: o_tx }).await.is_err() {
                    eprintln("Couldn't send site to main task!", &site).await;
                    return None;
                }

                match o_rx.await {
                    Ok(resp) if resp.to_process() => {
                        Some(StartTaskInfo {
                            site,
                            tx: self.tx.clone(),
                            client: self.client.clone(),
                        })
                    },
                    Ok(_) => None,
                    Err(_) => {
                        eprintln("Couldn't receive the response from main task!", &site).await;
                        None
                    }
                }
            })
            .collect().await;

            // Release semaphore before acquiring again
            drop(permit);

            for task_info in tmp {
                task_info.spawn_task(semaphore).await;
            }
        });
    }
}

pub struct SiteInfo {
    pub site: Arc<String>,
    pub responder: oneshot::Sender<Response>,
}

impl Debug for SiteInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.site)
    }
}

#[derive(Clone, Debug)]
pub struct Response {
    pub to_process: bool,
}

impl Response {
    pub fn to_process(&self) -> bool {
        self.to_process
    }
}