use hyper::client::connect::Connect;

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
