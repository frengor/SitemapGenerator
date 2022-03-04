#![allow(non_snake_case)]

use crate::input::Options;

mod utils;
mod site_analyzer;
mod input;

fn main() {
    let options = Options::from_cli();

    let sites_to_analyze = vec![
        "185.25.204.194",
        "https://docs.rs/hyper/0.14.16/hyper/client/struct.Client.html",
        "https://frengor.com/",
        "https://frengor.com/UltimateAdvancementAPI/",
    ];

    let sites_to_analyze = options.starting_points;

    let sites_to_analyze = sites_to_analyze.into_iter().map(|site| site.to_string()).collect();

    // Start tokio
    let sites = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .thread_name("SitemapGenerator")
    .build()
    .expect("Failed building the Runtime")
    .block_on(site_analyzer::analyze(sites_to_analyze, options.concurrent_tasks));

    for site in sites {
        println!("{}", site.as_ref());
    }

    if let Some(additional_links) = options.additional_links {
        additional_links.iter().map(|site| site.to_string()).for_each(|site| println!("{}", site));
    }
}


