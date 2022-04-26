#![allow(non_snake_case)]
#![forbid(unsafe_code)]

use std::fs::File;
use std::process::exit;
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
    .expect("Failed to build the Runtime")
    .block_on(sitemap_generator::analyze(sites_to_analyze.into_iter(), Validator::new(bin_options.sites_to_analyze.into_iter()), options));

    let mut i = 0usize;
    if bin_options.list_sites {
        for site in sites {
            i += 1; // sites is an iterator, cannot use len()
            println!("{}", &*site);
        }

        if let Some(additional_links) = bin_options.additional_links {
            i += additional_links.len();
            additional_links.iter().for_each(|site| println!("{}", site.as_str()));
        }
    } else if bin_options.print_total {
        i = sites.into_iter().count() + bin_options.additional_links.map_or(0usize, |links| links.len());
    } else if let Some(sitemap_file) = bin_options.sitemap_file {
        {
            // Empty the file
            if let Err(err) = File::create(&sitemap_file.sitemap_path) {
                eprintln!("Cannot create file {}. ({})", sitemap_file.sitemap_path.display(), err.kind());
                exit(1);
            }
        }
        if let Ok(mut file) = File::options().append(true).open(&sitemap_file.sitemap_path) {
            use std::io::Write;

            sites.into_iter().for_each(|site| {
                if let Err(err) = writeln!(file, "{}", site.as_str()) {
                    eprintln!("Cannot write to file {}. ({})", sitemap_file.sitemap_path.display(), err.kind());
                    exit(1);
                }
            });
        } else {
            eprintln!("Cannot open {}", sitemap_file.sitemap_path.display());
        }
    }

    if bin_options.print_total {
        println!("Done! ({})", i);
    }
}
