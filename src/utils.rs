use std::fmt::Display;

use hyper::client::connect::Connect;
use tokio::io::{AsyncWriteExt, stderr};
use url::Url;
use url_normalizer::normalize;

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

pub async fn eprintln(error: impl Display, site: &str) {
    let _ = stderr().write(format!(r#"An error occurred analyzing "{}": {error}\n"#, &site).as_bytes()).await;
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
        for url in self.iter.by_ref() {
            if let Ok(/*mut */normalized) = normalize(url) {
                /*normalized.set_query(None);
                normalized.set_fragment(None);*/
                return Some(normalized);
            }
        }
        None
    }
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
