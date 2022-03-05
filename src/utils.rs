use std::fmt::Display;

use hyper::client::connect::Connect;
use tokio::io::{AsyncWriteExt, stderr, stdout};
use url::Url;

/// Utility trait to apply trait bounds to generic parameters in functions which takes a `Client`.
///
/// The following code is more readable
/// ```
/// fn foo<T: ClientBounds>(client: Client<T>) {
///     // code here
/// }
///
/// foo(Client::new());
/// ```
/// than this
/// ```
/// fn foo<T>(client: Client<T>)
///     where T: Connect + Clone + Send + Sync + 'static
/// {
///     // code here
/// }
///
/// foo(Client::new());
/// ```
pub trait ClientBounds: Connect + Clone + Send + Sync + 'static {}

impl<T: Connect + Clone + Send + Sync + 'static> ClientBounds for T {}

pub async fn println(string: impl AsRef<str>) {
    let _ = stdout().write(string.as_ref().as_bytes()).await;
}

pub async fn eprintln(error: impl Display, site: &str) {
    let _ = stderr().write(format!("An error occurred analyzing \"{}\": {error}\n", site).as_bytes()).await;
}

pub trait Normalize: Iterator {
    fn normalize(self) -> Normalizer<Self> where Self: Sized;
}

impl<It: Iterator<Item=Url>> Normalize for It {
    #[inline]
    fn normalize(self) -> Normalizer<Self> where Self: Sized {
        Normalizer {
            iter: self,
        }
    }
}

pub struct Normalizer<It: Iterator> {
    iter: It,
}

impl<'it, It: Iterator<Item=Url>> Iterator for Normalizer<It> {
    type Item = Url;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(normalize)
    }
}

pub fn normalize(url: Url) -> Url {
    let url = url_normalizer::normalize_query(url);
    let url = url_normalizer::normalize_hash(url);
    url
}

#[macro_export]
macro_rules! measure_time {
    (nano: $($code:tt)*) => { $crate::__internal_measure_time!{use as_nanos for $( $code )*} };
    (micro: $($code:tt)*) => { $crate::__internal_measure_time!{use as_micros for $( $code )*} };
    (milli: $($code:tt)*) => { $crate::__internal_measure_time!{use as_millis for $( $code )*} };
    (sec: $($code:tt)*) => { $crate::__internal_measure_time!{use as_secs for $( $code )*} };
    ($($code:tt)*) => { $crate::__internal_measure_time!{use as_micros for $( $code )*} };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_measure_time {
    (use $time:ident for $($code:tt)*) => {{
        let now = ::std::time::Instant::now();
        let r = {
            $( $code )*
        };
        let elapsed = ::std::time::Duration::$time(&::std::time::Instant::elapsed(&now));
        ::std::println!("Elapsed time: {}", elapsed);
        r
    }};
}
