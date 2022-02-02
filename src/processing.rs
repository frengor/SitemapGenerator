use anyhow::{bail, Context, Result};
use hyper::Uri;
use scraper::Html;

use crate::{Client, ClientBounds};

pub async fn process<T: ClientBounds>(site: &str, client: Client<T>) -> Result<Html> {
    let uri: Uri = site.parse().context("invalid URL")?;

    let doc = fetch(uri, client).await?;
    Ok(Html::parse_document(&doc))
}

pub async fn fetch<T: ClientBounds>(uri: Uri, client: Client<T>) -> Result<String> {
    async fn get_content<T: ClientBounds>(uri: Uri, client: Client<T>) -> Result<String> {
        let resp = client.get(uri).await;

        let resp = match resp {
            Ok(resp) => resp,
            Err(e) => bail!("cannot make request: {e}"),
        };

        let bytes = hyper::body::to_bytes(resp.into_body()).await.context("cannot convert to bytes")?;
        Ok(String::from_utf8(bytes.to_vec()).context("cannot convert to String")?)
    }

    match uri.scheme_str() {
        Some("http") | Some("https") => get_content(uri, client).await,
        Some(x) => bail!("invalid URL protocol {x}"),
        None => bail!("missing protocol in URL"),
    }
}
