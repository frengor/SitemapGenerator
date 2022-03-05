use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use hyper::Uri;
use scraper::{Html, Selector};
use tokio::sync::oneshot;
use tokio::task::spawn_blocking;
use url::{Url};

use crate::site_analyzer::types::*;
use crate::utils::*;

pub async fn analyze_html<T: ClientBounds>(task_info: &StartTaskInfo<T>) -> Result<Vec<Url>> {
    let html_page = fetch(task_info.site.as_str(), task_info.client.clone()).await?;

    let (tx, rx) = oneshot::channel();
    let site = Arc::clone(&task_info.site);
    let validator = task_info.validator.clone();

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

        let links: Vec<Url> = html.select(&selector)
        .filter_map(|a_elem| a_elem.value().attr("href"))
        .filter_map(|link| match site.join(link) {
            Ok(link) => Some(link),
            Err(_) => None,
        })
        .filter(|url| match url.scheme() {
            "http" | "https" => true,
            _ => false,
        })
        .map(|mut url| {
            url.set_query(None);
            url.set_fragment(None);
            url
        })
        .normalize()
        .filter(|url| validator.is_valid(url))
        .collect();

        let _ = tx.send(Ok(links));
    });

    rx.await.with_context(|| format!("Cannot analyze site {}", &task_info.site))?
}

pub async fn fetch<T: ClientBounds>(site: &str, client: Client<T>) -> Result<String> {
    async fn get_content<T: ClientBounds>(uri: Uri, client: Client<T>) -> Result<String> {
        let resp = client.get(uri).await;

        let resp = match resp {
            Ok(resp) => resp,
            Err(e) => bail!("cannot make request: {e}"),
        };

        let bytes = hyper::body::to_bytes(resp.into_body()).await.context("cannot convert to bytes")?;
        Ok(String::from_utf8(bytes.to_vec()).context("cannot convert to String")?)
    }

    let uri: Uri = site.parse().context("invalid URL")?;

    match uri.scheme_str() {
        Some("http") | Some("https") => get_content(uri, client).await,
        Some(x) => bail!("invalid URL protocol {x}"),
        None => bail!("missing protocol in URL"),
    }
}
