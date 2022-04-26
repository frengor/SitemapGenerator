use std::collections::HashSet;
use std::fs;
use std::fs::{DirEntry, ReadDir};
use std::io::{stdin, stdout, Write};
use std::path::PathBuf;
use std::process::exit;

use clap::{CommandFactory, ErrorKind, Parser};
use faccess::PathExt;
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
    /// Whether to print verbose logging while crawling
    verbose: bool,
    #[clap(long)]
    /// If active, don't ask confirmations, just do it
    force: bool,

    // Output options
    #[clap(short = 'l', long = "list")]
    /// List the sites crawled to the standard output
    list_sites: bool,
    #[clap(short = 'F', long = "sitemap-file", group = "files", parse(from_os_str))]
    /// The file in which the sitemap will be put
    sitemap_path: Option<PathBuf>,
    #[clap(short = 'D', long = "additional-dir", requires = "files", parse(from_os_str))]
    /// The directory in which the additional sitemap files will be put if it is too big for one single file.
    /// THE DIRECTORY WILL BE CLEARED. WATCH OUT!
    additional_dir_path: Option<PathBuf>,
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
    pub(super) sitemap_file: Option<SitemapFileOptions>,
    pub(super) print_total: bool,
}

/// Sitemap file options
pub(super) struct SitemapFileOptions {
    /// Where the sitemap will be stored
    pub(super) sitemap_path: PathBuf,
    /// Where the additional sitemaps files will be stored if the sitemap is too big for one single file
    pub(super) additional_dir_path: Option<PathBuf>,
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

        ask_no_selected_output(&input);

        let file_opts = if let Some(path) = input.sitemap_path {
            // Try to open input.sitemap_path to make sure we can write to it
            if path.exists() && !path.writable() {
                error(format!("File {} cannot be opened for writing. (insufficient permissions)", path.display()));
            }

            // Try to open input.additional_dir_path to make sure we can read it
            if let Some(path) = &input.additional_dir_path {
                if path.exists() {
                    if !path.is_dir() {
                        error(format!(r#"Path "{}" is not a directory."#, path.display()));
                    }
                    match fs::read_dir(&path) {
                        Ok(read_dir) => ask_directory_with_content(read_dir, input.force, path),
                        Err(err) => error(format!(r#"Error reading directory "{}". ({})"#, path.display(), err.kind())),
                    }

                    if !path.writable() {
                        error(format!("Directory {} cannot be opened for writing. (insufficient permissions)", path.display()));
                    }
                } else if let Err(err) = fs::create_dir_all(&path) {
                    error(format!("Directory {} cannot be created. ({})", path.display(), err.kind()));
                }
            }
            Some(SitemapFileOptions {
                sitemap_path: path,
                additional_dir_path: input.additional_dir_path,
            })
        } else {
            None
        };

        let mut bin_options = BinOptions {
            sites_to_analyze: input.sites_to_analyze.iter().map(|str| sites_to_analyze_validator(str)).collect(),
            starting_points: input.starting_points.map_or(HashSet::new(), |vec| vec.iter().map(|str| url_validator(str)).collect()),
            additional_links: input.additional_links.map(|vec| vec.iter().map(|str| url_parser(str)).collect()),

            list_sites: input.list_sites,
            sitemap_file: file_opts,
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

fn ask_directory_with_content(mut read_dir: ReadDir, force: bool, additional_dir_path: &PathBuf) {
    let first_content = read_dir.next();

    fn empty_dir(file: std::io::Result<DirEntry>, read_dir: ReadDir) {
        fn remove_file(path: PathBuf) {
            match () {
                () if path.is_file() || path.is_symlink() => {
                    let _ = fs::remove_file(path);
                },
                () if path.is_dir() => {
                    let _ = fs::remove_dir_all(path);
                },
                _ => {},
            }
        }
        if let Ok(file) = file {
            remove_file(file.path());
        }
        read_dir.for_each(|e| {
            if let Ok(f) = e {
                remove_file(f.path());
            }
        });
    }

    match first_content {
        Some(elem) if force => empty_dir(elem, read_dir),
        Some(elem) if !force => {
            print!("Selected directory ({}) contains files, proceed? [Y/n] ", additional_dir_path.display());
            let _ = stdout().flush();
            confirmation();
            empty_dir(elem, read_dir);
        },
        _ => {}, // Dir is already empty
    }
}

fn ask_no_selected_output(input: &Input) {
    if let (false, false, None, false) = (input.force, input.list_sites, &input.sitemap_path, input.print_total) {
        print!("No output option has been selected, proceed? [Y/n] ");
        let _ = stdout().flush();
        confirmation();
    }
}

/// Quits if user says "no"
fn confirmation() {
    let mut str = String::with_capacity(8 /*Just to be sure we don't reallocate for every input*/);
    if let Err(err) = stdin().read_line(&mut str) {
        eprintln!("Cannot get user input: {}", err.kind());
        exit(1);
    }
    str.make_ascii_lowercase();
    match str.trim() {
        "" | "y" | "yes" => {
            println!("Continuing...");
        },
        _ => {
            println!("Aborting...");
            exit(0);
        },
    }
}

#[test]
fn verify_app() {
    use clap::CommandFactory;
    Input::command().debug_assert();
}
