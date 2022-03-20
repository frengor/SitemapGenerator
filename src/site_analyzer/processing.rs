use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use lazy_static::lazy_static;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::sync::{oneshot, Semaphore};
use tokio::task::spawn_blocking;
use url::Url;

use crate::{Options, TaskInfo, Validator};
use crate::utils::*;

lazy_static! {
    static ref A_SELECTOR: Selector = Selector::parse("a").unwrap();
    static ref BASE_SELECTOR: Selector = Selector::parse("base").unwrap();
}

async fn make_request(task_info: &TaskInfo, client: Client) -> Result<String> {
    match client.get((*task_info.site).clone()).build() {
        Ok(request) => {
            Ok(client.execute(request).await?.text().await?)
        },
        Err(err) => Err(anyhow!(err)),
    }
}

pub async fn analyze_html(task_info: &TaskInfo, client: Client, semaphore: &Semaphore, options: &Options) -> Result<Vec<Url>> {
    if options.verbose() {
        if let Some(tx) = options.verbose_sender() {
            let _ = tx.send(task_info.site.clone());
        }
    }
    let html_page = make_request(task_info, client).await?;

    let (tx, rx) = oneshot::channel();
    let site = Arc::clone(&task_info.site);
    let validator = task_info.validator.clone();
    let remove_query_and_fragment = options.remove_query_and_fragment();

    let permit = match semaphore.acquire().await {
        Ok(permit) => permit,
        Err(_) => bail!("cannot spawn task"),
    };

    spawn_blocking(move || {
        let html = Html::parse_document(&html_page);

        let base_url = html.select(&*BASE_SELECTOR)
        .filter_map(|element| element.value().attr("href"))
        .map(Url::parse)
        .filter_map(Result::ok)
        .filter_http()
        .filter(|url| !url.cannot_be_a_base())
        .normalize()
        .next();
        // Splitting this in two to make code compile
        let base_url = base_url.as_ref().unwrap_or(&*site);

        let iter = html.select(&*A_SELECTOR)
        .filter_map(|a_elem| a_elem.value().attr("href"))
        .filter_map(|link| base_url.join(link).ok())
        .filter_http();

        fn finish_collecting(iter: impl Iterator<Item=Url>, validator: &Validator) -> Vec<Url> {
            iter.normalize()
            .filter(|url| validator.is_valid(url))
            .collect()
        }

        let links = if remove_query_and_fragment {
            finish_collecting(iter.map(|mut url| {
                url.set_query(None);
                url.set_fragment(None);
                url
            }), &validator)
        } else {
            finish_collecting(iter, &validator)
        };

        let _ = tx.send(Ok(links));
    });

    let result = rx.await;

    // Release semaphore
    drop(permit);

    result.with_context(|| format!("Cannot analyze site {}", &task_info.site))?
}
