use std::collections::HashSet;

use clap::{CommandFactory, ErrorKind, Parser};
use url::{Position, Url};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Input {
    domains_to_analyze: Vec<String>,
    #[clap(short, long)]
    starting_points: Option<Vec<String>>,
    #[clap(short, long)]
    additional_links: Option<Vec<String>>,
    #[clap(short, long, default_value_t = num_cpus::get())]
    concurrent_tasks: usize,
}

pub(super) struct Options {
    pub domains_to_analyze: HashSet<Url>,
    pub starting_points: HashSet<Url>,
    pub additional_links: Option<HashSet<Url>>,
    pub concurrent_tasks: usize,
}

impl Options {
    #[inline]
    pub(super) fn from_cli() -> Options {
        Input::parse().into()
    }
}

impl From<Input> for Options {
    fn from(input: Input) -> Self {
        if input.domains_to_analyze.is_empty() {
            error("No domain has been provided.".to_string());
        }
        if input.concurrent_tasks == 0 {
           error("Concurrent tasks must be greater than zero.".to_string());
        }
        let mut opts = Options {
            domains_to_analyze: input.domains_to_analyze.iter().map(|str| domain_validator(str)).collect(),
            starting_points: input.starting_points.map_or(HashSet::new(), |vec| vec.iter().map(|str| url_validator(str)).collect()),
            additional_links: input.additional_links.map(|vec| vec.iter().map(|str| url_parser(str)).collect()),
            concurrent_tasks: input.concurrent_tasks,
        };
        opts.domains_to_analyze.iter().for_each(|url| { opts.starting_points.insert(url.clone()); });
        opts
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
    match Url::parse(url) {
        Ok(url) => url,
        Err(parse_err) => error(parse_err.to_string()),
    }
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
