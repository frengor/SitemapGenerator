#![allow(non_snake_case)]

use tokio::sync::OnceCell;
use crate::input::Options;
use crate::site_analyzer::types::Validator;

mod utils;
mod site_analyzer;
mod input;

static OPTIONS: OnceCell<Options> = OnceCell::const_new();

pub fn get_options() -> &'static Options {
    OPTIONS.get().unwrap()
}

fn main() {
    let mut options = Options::from_cli();

    let _sites_to_analyze = vec![
        "185.25.204.194",
        "https://docs.rs/hyper/0.14.16/hyper/client/struct.Client.html",
        "https://frengor.com/",
        "https://frengor.com/UltimateAdvancementAPI/",
    ];

    let other_options = options.other_options().unwrap();

    if OPTIONS.set(options).is_err() {
        panic!("Cannot set OPTIONS");
    }

    let sites_to_analyze = other_options.starting_points;

    // Start tokio
    let sites = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .thread_name("SitemapGenerator")
    .build()
    .expect("Failed building the Runtime")
    .block_on(site_analyzer::analyze(sites_to_analyze.into_iter(), Validator::new(other_options.domains_to_analyze.into_iter())));

    for site in sites {
        println!("{}", site.as_ref());
    }

    if let Some(additional_links) = other_options.additional_links {
        additional_links.iter().map(|site| site.to_string()).for_each(|site| println!("{}", site));
    }
}


