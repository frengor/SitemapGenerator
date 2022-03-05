use std::fmt::{Debug, Formatter};
use std::sync::Arc;

use async_recursion::async_recursion;
use futures::{stream, StreamExt};
use hyper::Body;
use tokio::sync::{oneshot, Semaphore};
use tokio::sync::mpsc::Sender;
use url::Url;

use crate::utils::*;

use super::processing;

pub type Client<T> = hyper::Client<T, Body>;

pub struct StartTaskInfo<T: ClientBounds> {
    pub site: Arc<Url>,
    pub tx: Sender<SiteInfo>,
    pub client: Client<T>,
    pub validator: Validator,
    pub recursion: usize,
}

impl<T: ClientBounds> StartTaskInfo<T> {
    #[async_recursion]
    pub async fn spawn_task(self, semaphore: &'static Semaphore) {
        if self.recursion == 0 {
           return;
        }
        let permit = match semaphore.acquire().await {
            Ok(permit) => permit,
            Err(_) => {
                eprintln("cannot spawn task", self.site.as_str()).await;
                return;
            }
        };

        tokio::spawn(async move {
            let links = match processing::analyze_html(&self).await {
                Ok(links) => links,
                Err(err) => {
                    eprintln(err, self.site.as_str()).await;
                    return;
                },
            };

            let tmp: Vec<_> = stream::iter(links)
            .filter_map(|link| async {
                let site = Arc::new(link);
                let (o_tx, o_rx) = oneshot::channel();
                if self.tx.send(SiteInfo { site: Arc::clone(&site), responder: o_tx }).await.is_err() {
                    eprintln("Couldn't send site to main task!", site.as_str()).await;
                    return None;
                }

                match o_rx.await {
                    Ok(resp) if resp.to_process() => {
                        Some(StartTaskInfo {
                            site,
                            tx: self.tx.clone(),
                            client: self.client.clone(),
                            validator: self.validator.clone(),
                            recursion: self.recursion - 1,
                        })
                    },
                    Ok(_) => None,
                    Err(_) => {
                        eprintln("Couldn't receive the response from main task!", site.as_str()).await;
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
    pub site: Arc<Url>,
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
    #[inline]
    pub fn to_process(&self) -> bool {
        self.to_process
    }
}

#[derive(Debug, Clone)]
pub struct Validator {
    // Using an Arc to allow cloning
    domains: Arc<Vec<String>>,
}

impl Validator {
    pub fn new(iter: impl Iterator<Item=Url>) -> Validator {
        Validator {
            domains: Arc::new(iter
            .filter_map(|url| url.host_str().map(|url| url.to_string()))
            .collect()),
        }
    }

    pub fn is_valid(&self, url: &Url) -> bool {
        let host_str = url.host_str();
        self.domains.iter().any(|domain| {
            host_str.map_or(false, |host| host == domain)
        })
    }
}