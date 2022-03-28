#![allow(non_snake_case)]
#![forbid(unsafe_code)]

pub use sitemap_generator::{Options, utils, Validator};

mod input;

fn main() {
    let (options, bin_options) = input::from_cli();

    let sites_to_analyze = bin_options.starting_points;

    // Start tokio
    let sites = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .thread_name("SitemapGenerator")
    .build()
    .expect("Failed building the Runtime")
    .block_on(sitemap_generator::analyze(sites_to_analyze.into_iter(), Validator::new(bin_options.sites_to_analyze.into_iter()), options));

    let mut i = 0usize;
    for site in sites {
        i += 1; // sites is an iterator, cannot use len()
        println!("{}", &*site);
    }

    if let Some(additional_links) = bin_options.additional_links {
        i += additional_links.len();
        additional_links.iter().for_each(|site| println!("{}", site.as_str()));
    }

    if bin_options.print_total {
        println!("Done! ({})", i);
    }
}
