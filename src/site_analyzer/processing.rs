use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use scraper::{Html, Selector};
use tokio::sync::{oneshot, Semaphore};
use tokio::task::spawn_blocking;
use url::Url;

use crate::{Options, StartTaskInfo, Validator};
use crate::utils::*;

pub async fn analyze_html(task_info: &StartTaskInfo, semaphore: &Semaphore, options: &Options) -> Result<Vec<Url>> {
    let html_page = reqwest::get((*task_info.site).clone()).await?.text().await?;

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

        let selector = match Selector::parse("a") {
            Ok(selector) => selector,
            Err(err) => {
                // Exit with error
                let _ = tx.send(Err(anyhow!("cannot parse selector: {:?}", err.kind)));
                return;
            },
        };

        let iter = html.select(&selector)
        .filter_map(|a_elem| a_elem.value().attr("href"))
        .filter_map(|link| match site.join(link) {
            Ok(link) => Some(link),
            Err(_) => None,
        })
        .filter(|url| matches!(url.scheme(), "http" | "https"));

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
