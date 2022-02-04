#![allow(non_snake_case)]

mod utils;
mod site_analyzer;

const CONCURRENT_TASKS: usize = 64;

fn main() {
    let sites_to_analyze = vec![
        "185.25.204.194",
        "https://docs.rs/hyper/0.14.16/hyper/client/struct.Client.html",
        "https://frengor.com/",
        "https://frengor.com/UltimateAdvancementAPI/",
    ];

    let sites_to_analyze = sites_to_analyze.into_iter().map(|site| site.to_string()).collect();

    // Start tokio
    let sites = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .thread_name("SitemapGenerator")
    .build()
    .expect("Failed building the Runtime")
    .block_on(site_analyzer::analyze(sites_to_analyze, CONCURRENT_TASKS));

    for site in sites {
        println!("{}", site.as_ref());
    }
}


