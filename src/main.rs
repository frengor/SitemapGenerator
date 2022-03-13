#![allow(non_snake_case)]
#![forbid(unsafe_code)]

mod input;

pub use sitemap_generator::{utils, Options, Validator};

fn main() {
    let (options, other_options) = input::from_cli();

    let _sites_to_analyze = vec![
        "185.25.204.194",
        "https://docs.rs/hyper/0.14.16/hyper/client/struct.Client.html",
        "https://frengor.com/",
        "https://frengor.com/UltimateAdvancementAPI/",
    ];

    let sites_to_analyze = other_options.starting_points;

    // Start tokio
    let sites = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .thread_name("SitemapGenerator")
    .build()
    .expect("Failed building the Runtime")
    .block_on(sitemap_generator::analyze(sites_to_analyze.into_iter(), Validator::new(other_options.sites_to_analyze.into_iter()), options));

    for site in sites {
        println!("{}", site.as_ref());
    }

    if let Some(additional_links) = other_options.additional_links {
        additional_links.iter().map(|site| site.to_string()).for_each(|site| println!("{}", site));
    }
}
