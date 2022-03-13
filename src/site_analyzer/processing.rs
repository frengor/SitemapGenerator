use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use lazy_static::lazy_static;
use reqwest::Client;
use scraper::{Html, Selector};
use tokio::sync::{oneshot, Semaphore};
use tokio::task::spawn_blocking;
use url::Url;

use crate::{Options, StartTaskInfo, Validator};
use crate::utils::*;

lazy_static! {
    static ref A_SELECTOR: Selector = Selector::parse("a").unwrap();
    static ref BASE_SELECTOR: Selector = Selector::parse("base").unwrap();
}

pub async fn analyze_html(task_info: &StartTaskInfo, client: Client, semaphore: &Semaphore, options: &Options) -> Result<Vec<Url>> {
    let html_page = match client.get((*task_info.site).clone()).build() {
        Ok(request) => {
            client.execute(request).await?.text().await?
        },
        Err(err) => {
            return Err(anyhow!(err));
        },
    };

    if options.verbose() {
        let site = task_info.site.clone();
        tokio::spawn({
            println(format!("Analyzing: \"{}\"\n", site.as_str()))
        });
    }

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
        .filter_map(|result| result.ok())
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

    let result = rx.await.with_context(|| format!("Cannot analyze site {}", &task_info.site))?;

    // Release semaphore
    drop(permit);

    result
}
