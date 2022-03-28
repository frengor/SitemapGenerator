use std::collections::HashSet;
use std::fmt::format;
use std::fs;
use std::path::PathBuf;

use clap::{CommandFactory, ErrorKind, Parser};
use url::Url;

use sitemap_generator::Options;

use crate::utils::*;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Input {
    /// List of sites to analyze. Example: https://frengor.com
    sites_to_analyze: Vec<String>,
    #[clap(short, long)]
    /// Sites to start the crawl from. Contains DOMAINS_TO_ANALYZE by default (add --sdasp to disable)
    starting_points: Option<Vec<String>>,
    #[clap(long = "sstaasp")]
    /// Skip Sites to Analyze As Starting Points
    skip_sites_to_analyze_as_starting_points: bool,
    #[clap(short, long)]
    /// List of links to add to the sitemap, but not to crawl
    additional_links: Option<Vec<String>>,
    #[clap(short = 'c', long, default_value_t = num_cpus::get())]
    /// Max number of sites analyzed simultaneously. Default value is the number of CPU cores
    max_concurrent_tasks: usize,
    #[clap(long = "rqaf")]
    /// Remove Query And Fragment from the analyzed urls
    remove_query_and_fragment: bool,
    #[clap(short = 'd', long, default_value_t = 50)]
    /// Max depth of the crawl. Default value is 50
    max_depth: usize,
    #[clap(short, long)]
    verbose: bool,

    // Output options
    #[clap(short = 'l', long = "list")]
    /// List the sites crawled to the standard output
    list_sites: bool,
    #[clap(short, long, parse(from_os_str))]
    /// List the sites crawled to the standard output
    file: PathBuf,
    #[clap(short = 't', long = "total")]
    /// Print the total amount of sites crawled
    print_total: bool,
}

/// Options of bin version of sitemap_generator, lib options are inside Options struct
pub(super) struct BinOptions {
    pub(super) sites_to_analyze: HashSet<Url>,
    pub(super) starting_points: HashSet<Url>,
    pub(super) additional_links: Option<HashSet<Url>>,

    pub(super) list_sites: bool,
    pub(super) file: PathBuf,
    pub(super) print_total: bool,
}

#[inline]
pub(super) fn from_cli() -> (Options, BinOptions) {
    Input::parse().into()
}

impl From<Input> for (Options, BinOptions) {
    fn from(input: Input) -> Self {
        if input.sites_to_analyze.is_empty() {
            error("No domain has been provided.".to_string());
        }
        if input.max_concurrent_tasks == 0 {
            error("Concurrent tasks must be greater than zero.".to_string());
        }

        {
            // Try to open input.file to make sure we can write to it.
            let open_options = fs::OpenOptions::new().write(true).create_new(true).open(&input.file);
            if open_options.is_err() {
                error(format!("File {} cannot be opened.", input.file.display()));
            }
        }

        let mut bin_options = BinOptions {
            sites_to_analyze: input.sites_to_analyze.iter().map(|str| sites_to_analyze_validator(str)).collect(),
            starting_points: input.starting_points.map_or(HashSet::new(), |vec| vec.iter().map(|str| url_validator(str)).collect()),
            additional_links: input.additional_links.map(|vec| vec.iter().map(|str| url_parser(str)).collect()),

            list_sites: input.list_sites,
            file: input.file,
            print_total: input.print_total,
        };
        bin_options.sites_to_analyze.iter().for_each(|url| { bin_options.starting_points.insert(url.clone()); });

        let options = Options::new(input.max_concurrent_tasks, input.remove_query_and_fragment, input.max_depth, input.verbose);
        (options, bin_options)
    }
}

#[inline]
fn error(error: String) -> ! {
    Input::command().error(
        ErrorKind::InvalidValue,
        error,
    ).exit()
}

fn url_parser(url: &str) -> Url {
    let parsed_url = match Url::parse(url) {
        Ok(url) => url,
        Err(parse_err) => error(format!(r#"Error parsing url "{}": {}"#, url, parse_err)),
    };
    normalize(parsed_url)
}

fn url_validator(url: &str) -> Url {
    let url = url_parser(url);
    if url.cannot_be_a_base() {
        error(format!(r#""{url}" is not an abstract URL"#))
    }
    url
}

fn sites_to_analyze_validator(url: &str) -> Url {
    let site_to_analyze = url_validator(url);
    {
        let sub_url = site_to_analyze.clone();
        let check = |str: Option<&str>| {
            match str {
                None | Some("") => (),
                _ => { error(format!(r#""{url}" is an invalid site to analyze"#)); },
            }
        };

        check(sub_url.query());
        check(sub_url.fragment());
    }
    site_to_analyze
}

#[test]
fn verify_app() {
    use clap::CommandFactory;
    Input::command().debug_assert();
}
