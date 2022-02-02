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