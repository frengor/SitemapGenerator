use std::fmt::Debug;
use std::sync::Arc;

use futures::{stream, StreamExt};
use reqwest::Client;
use tokio::sync::mpsc::Sender;
use tokio::sync::Semaphore;
use url::Url;

use crate::Options;
use crate::site_analyzer::processing::analyze_html;
use crate::utils::*;

pub struct StartTaskInfo {
    pub site: Arc<Url>,
    pub tx: Sender<StartTaskInfo>,
    pub validator: Validator,
    pub recursion: usize,
}

impl StartTaskInfo {
    pub async fn spawn_task(self, client: Client, semaphore: Arc<Semaphore>, options: Arc<Options>) {
        if self.recursion == 0 {
            return;
        }

        tokio::spawn(async move {
            let links = match analyze_html(&self, client, &semaphore, &options).await {
                Ok(links) => links,
                Err(err) => {
                    eprintln(err, self.site.as_str()).await;
                    return;
                },
            };

            stream::iter(links)
            .for_each(|link| async {
                let site = Arc::new(link);
                let start_task_info = StartTaskInfo {
                    site: site.clone(),
                    tx: self.tx.clone(),
                    validator: self.validator.clone(),
                    recursion: self.recursion - 1,
                };
                if self.tx.send(start_task_info).await.is_err() {
                    eprintln("Couldn't send site to main task!", site.as_str()).await;
                }
            }).await;
        });
    }
}

#[derive(Debug, Clone)]
pub struct Validator {
    // Using an Arc to allow cloning
    base_urls: Arc<Vec<String>>,
}

impl Validator {
    pub fn new(iter: impl Iterator<Item=Url>) -> Validator {
        Validator {
            base_urls: Arc::new(iter
            .map(|mut url| {
                url.set_query(None);
                url.set_fragment(None);
                url
            })
            .filter(|url| !url.cannot_be_a_base())
            .map(String::from)
            .collect()),
        }
    }

    pub fn is_valid(&self, url: &Url) -> bool {
        let str = url.as_str();
        self.base_urls.iter().any(|base_url| {
            str.starts_with(base_url)
        })
    }
}
