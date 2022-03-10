use std::fmt::Display;
use std::iter::Map;

use tokio::io::{AsyncWriteExt, stderr, stdout};
use url::Url;

#[inline]
pub async fn println(string: impl AsRef<str>) {
    let _ = stdout().write(string.as_ref().as_bytes()).await;
}

#[inline]
pub async fn eprintln(error: impl Display, site: &str) {
    let _ = stderr().write(format!("An error occurred analyzing \"{}\": {error}\n", site).as_bytes()).await;
}

pub trait Normalize: Iterator<Item=Url> {
    #[inline]
    fn normalize(self) -> Map<Self, fn(Url) -> Url>
    where
        Self: Sized
    {
        self.map(normalize)
    }
}

impl<It: Iterator<Item=Url>> Normalize for It {}

#[inline]
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
