use std::collections::HashSet;
use std::mem;

use clap::{CommandFactory, ErrorKind, Parser};
use url::{Position, Url};

use crate::utils::*;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Input {
    /// List of domains to analyze. Example: https://frengor.com
    domains_to_analyze: Vec<String>,
    #[clap(short, long)]
    /// Sites to start the crawl from. Contains DOMAINS_TO_ANALYZE by default (add --sdasp to disable)
    starting_points: Option<Vec<String>>,
    #[clap(long = "sdasp")]
    /// Skip Domains As Starting Points
    skip_domains_as_starting_points: bool,
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
}

pub(super) struct OtherOptions {
    pub(super) domains_to_analyze: HashSet<Url>,
    pub(super) starting_points: HashSet<Url>,
    pub(super) additional_links: Option<HashSet<Url>>,
}

pub struct Options {
    other_options: Option<OtherOptions>,
    max_task_count: usize,
    remove_query_and_fragment: bool,
    max_recursion: usize,
    verbose: bool,
}

impl Options {
    #[inline]
    pub(super) fn from_cli() -> Options {
        Input::parse().into()
    }

    pub(super) fn other_options(&mut self) -> Option<OtherOptions> {
        if self.other_options.is_some() {
            let mut tmp: Option<OtherOptions> = None;
            mem::swap(&mut self.other_options, &mut tmp);
            Some(tmp.unwrap())
        } else {
            None
        }
    }

    #[inline]
    pub fn max_task_count(&self) -> usize {
        self.max_task_count
    }

    #[inline]
    pub fn remove_query_and_fragment(&self) -> bool {
        self.remove_query_and_fragment
    }

    #[inline]
    pub fn max_recursion(&self) -> usize {
        self.max_recursion
    }

    #[inline]
    pub fn verbose(&self) -> bool {
        self.verbose
    }
}

impl From<Input> for Options {
    fn from(input: Input) -> Self {
        if input.domains_to_analyze.is_empty() {
            error("No domain has been provided.".to_string());
        }
        if input.max_concurrent_tasks == 0 {
            error("Concurrent tasks must be greater than zero.".to_string());
        }

        let mut other_options = OtherOptions {
            domains_to_analyze: input.domains_to_analyze.iter().map(|str| domain_validator(str)).collect(),
            starting_points: input.starting_points.map_or(HashSet::new(), |vec| vec.iter().map(|str| url_validator(str)).collect()),
            additional_links: input.additional_links.map(|vec| vec.iter().map(|str| url_parser(str)).collect()),
        };
        other_options.domains_to_analyze.iter().for_each(|url| { other_options.starting_points.insert(url.clone()); });

        Options {
            other_options: Some(other_options),
            max_task_count: input.max_concurrent_tasks,
            remove_query_and_fragment: input.remove_query_and_fragment,
            max_recursion: input.max_depth,
            verbose: input.verbose,
        }
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
        Err(parse_err) => error(parse_err.to_string()),
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

fn domain_validator(domain: &str) -> Url {
    let domain = url_validator(domain);
    {
        let sub_url = &domain[Position::BeforePath..];
        if !(sub_url.is_empty() || sub_url == "/") {
            error(format!(r#""{domain}" is not a valid domain"#))
        }
    }
    domain
}

#[test]
fn verify_app() {
    use clap::CommandFactory;
    Input::command().debug_assert();
}
