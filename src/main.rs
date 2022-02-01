#![allow(non_snake_case)]

use anyhow::{bail, Context, Result};
use futures::{stream, StreamExt};
use hyper::Uri;
use hyper_tls::HttpsConnector;

pub use crate::utils::*;

mod utils;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder().build(HttpsConnector::new());

    let sites = vec![
        "185.25.204.194",
        "https://docs.rs/hyper/0.14.16/hyper/client/struct.Client.html",
        "https://frengor.com/",
        "https://frengor.com/UltimateAdvancementAPI/",
    ];
    let len = sites.len();

    // https://stackoverflow.com/a/51047786
    stream::iter(sites)
    .map(|site| {
        let client = client.clone();
        tokio::spawn(async move {
            (site, analyze_url(site, client).await)
        })
    })
    .buffer_unordered(len)
    .for_each(|future| async {
        match future {
            Ok((site, Ok(content))) => println!("{}:\n{}\n\n", site, content.chars().take(25).collect::<String>()),
            Ok((site, Err(e))) => eprintln!(r#"An error occurred analyzing "{site}": {e}"#),
            Err(e) => eprintln!(r#"A Tokio error occurred: {e}"#),
        }
    })
    .await;
    Ok(())
}

async fn analyze_url<T: ClientBounds>(site: &str, client: Client<T>) -> Result<String> {
    let uri: Uri = site.parse().context("invalid URL")?;

    match uri.scheme_str() {
        Some("http") | Some("https") => get_content(uri, client).await,
        Some(x) => bail!("invalid URL protocol {x}"),
        None => bail!("missing protocol in URL"),
    }
}

async fn get_content<T: ClientBounds>(uri: Uri, client: Client<T>) -> Result<String> {
    let resp = client.get(uri).await;

    let resp = match resp {
        Ok(resp) => resp,
        Err(e) => bail!("cannot make request: {e}"),
    };

    let bytes = hyper::body::to_bytes(resp.into_body()).await.context("cannot convert to bytes")?;
    Ok(String::from_utf8(bytes.to_vec()).context("cannot convert to String")?)
}
