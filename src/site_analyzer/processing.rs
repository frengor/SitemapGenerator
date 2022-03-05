use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Error, Result};
use hyper::{StatusCode, Uri};
use hyper::header::LOCATION;
use scraper::{Html, Selector};
use tokio::sync::oneshot;
use tokio::task::spawn_blocking;
use url::Url;

use crate::get_options;
use crate::site_analyzer::types::*;
use crate::utils::*;

pub async fn analyze_html<T: ClientBounds>(task_info: &StartTaskInfo<T>) -> Result<Vec<Url>> {
    let html_page = fetch(task_info.site.as_str(), task_info.client.clone()).await?;
    let options = get_options();
    if options.verbose() {
        println(format!("Analyzing: \"{}\"\n", task_info.site.as_str())).await;
    }

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

        let links = if options.remove_query_and_fragment() {
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

    rx.await.with_context(|| format!("Cannot analyze site {}", &task_info.site))?
}

pub async fn fetch<T: ClientBounds>(site: &str, client: Client<T>) -> Result<String> {
    async fn parse_uri(url: &str) -> Result<Uri, Error> {
        let uri: Uri = url.parse().context("invalid URL")?;

        match uri.scheme_str() {
            Some("http") | Some("https") => Ok(uri),
            Some(x) => bail!("invalid URL protocol {x}"),
            None => bail!("missing protocol in URL"),
        }
    }

    async fn get_content<T: ClientBounds>(uri: Uri, client: Client<T>) -> Result<String> {
        async fn make_request<T: ClientBounds>(uri: Uri, client: &Client<T>) -> Result<(hyper::Response<hyper::Body>, StatusCode)> {
            let resp = client.get(uri).await;

            let resp = match resp {
                Ok(resp) => resp,
                Err(e) => bail!("cannot make request: {e}"),
            };
            let status = resp.status();
            Ok((resp, status))
        }
        let mut resp = make_request(uri, &client).await?;
        while resp.1.is_redirection() {
            let uri = match resp.0.headers().get(LOCATION) {
                Some(header) => header.to_str(),
                None => bail!("Received {} code (redirect), but no Location header has been found.", resp.1.as_u16()),
            }?;
            resp = make_request(parse_uri(uri).await?, &client).await?;
        }
        let bytes = hyper::body::to_bytes(resp.0.into_body()).await.context("cannot convert to bytes")?;
        Ok(String::from_utf8(bytes.to_vec()).context("cannot convert to String")?)
    }

    get_content(parse_uri(site).await?, client).await
}
