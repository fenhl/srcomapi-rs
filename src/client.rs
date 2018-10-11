//! The `Client` type is the entry point to the API.

use std::{
    fmt,
    marker::PhantomData
};
use reqwest::{
    self,
    RequestBuilder
};
use super::Result;

static BASE_URL: &'static str = "https://www.speedrun.com/api/v1";

/// A marker type used as a type parameter on `Client` to indicate that the client is authenticated.
#[derive(Debug, Clone, Copy)]
pub enum Auth {}

/// A marker type used as a type parameter on `Client` to indicate that the client is not authenticated. This is the default.
#[derive(Debug, Clone, Copy)]
pub enum NoAuth {}

/// The entry point to the API.
///
/// The client automatically inserts pauses between requests if necessary according to the API's [rate limits](https://github.com/speedruncomorg/api/blob/master/throttling.md). However, this only works if your application uses the same `Client` for all API requests. If you use multiple `Client`s, you risk getting HTTP `420` errors due to rate limiting.
#[derive(Debug, Clone)]
pub struct Client<A = NoAuth> {
    client: reqwest::Client,
    phantom: PhantomData<A>
}

impl Client<NoAuth> {
    /// Constructs a new `Client` for accessing the API without authenticating as a user.
    ///
    /// The `user_agent` parameter is used as the `User-Agent` header for all requests. It must be a `&'static str` for performance reasons. To quote the API docs:
    ///
    /// > If possible, please set a descriptive `User-Agent` HTTP header. This makes it easier for us to see how the API is being used and optimise it further. A good user agent string includes your project name and possibly the version number, like `my-bot/4.20`.
    ///
    /// # Errors
    ///
    /// This method fails if native TLS backend cannot be initialized.
    ///
    /// # Panics
    ///
    /// This method panics if the user agent contains invalid [header value characters](https://docs.rs/reqwest/*/reqwest/header/struct.HeaderValue.html#method.from_static).
    pub fn new(user_agent: &'static str) -> Result<Client> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::USER_AGENT, reqwest::header::HeaderValue::from_static(user_agent));
        Ok(Client {
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()?,
            phantom: PhantomData
        })
    }
}

impl Client<Auth> {
    /// Constructs a new `Client` for accessing the API and authenticates a user, so that all requests are made as that user.
    ///
    /// For details on obtaining a user's API key, see [the docs on authentication](https://github.com/speedruncomorg/api/blob/master/authentication.md).
    ///
    /// The `user_agent` parameter is used as the `User-Agent` header for all requests. It must be a `&'static str` for performance reasons. To quote the API docs:
    ///
    /// > If possible, please set a descriptive `User-Agent` HTTP header. This makes it easier for us to see how the API is being used and optimise it further. A good user agent string includes your project name and possibly the version number, like `my-bot/4.20`.
    ///
    /// # Errors
    ///
    /// This method fails if native TLS backend cannot be initialized or the API key contains invalid [header value characters](https://docs.rs/reqwest/*/reqwest/header/struct.HeaderValue.html#method.from_static).
    ///
    /// # Panics
    ///
    /// This method panics if the user agent contains invalid [header value characters](https://docs.rs/reqwest/*/reqwest/header/struct.HeaderValue.html#method.from_static).
    pub fn new(user_agent: &'static str, api_key: &str) -> Result<Client<Auth>> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::USER_AGENT, reqwest::header::HeaderValue::from_static(user_agent));
        headers.insert("X-API-Key", reqwest::header::HeaderValue::from_str(api_key)?);
        Ok(Client {
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()?,
            phantom: PhantomData
        })
    }
}

impl<A> Client<A> {
    pub(crate) fn get(&self, url: impl fmt::Display) -> RequestBuilder {
        //TODO wait for rate limit
        self.client.get(&format!("{}{}", BASE_URL, url))
    }
}

impl<A: Clone> Client<A> {
    pub(crate) fn annotate<T>(&self, data: T) -> AnnotatedData<T, A> {
        AnnotatedData {
            data,
            client: self.clone()
        }
    }
}

impl From<Client<Auth>> for Client<NoAuth> {
    fn from(auth_client: Client<Auth>) -> Client<NoAuth> {
        Client {
            client: auth_client.client,
            phantom: PhantomData
        }
    }
}

impl<'a, A: Clone> From<&'a Client<A>> for Client<A> {
    fn from(client_ref: &Client<A>) -> Client<A> {
        client_ref.clone()
    }
}

impl<'a> From<&'a Client<Auth>> for Client<NoAuth> {
    fn from(auth_client_ref: &Client<Auth>) -> Client<NoAuth> {
        Client::<Auth>::from(auth_client_ref).into()
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ResponseData<T> {
    pub(crate) data: T
}

/// This type is an implementation detail.
///
/// It is a helper type which includes data of some sort, as well as a copy of the client to make further API requests. Most API methods are defined on `AnnotatedData<T>` instances for some concrete `T`.
#[derive(Debug, Clone)]
pub struct AnnotatedData<T, A = NoAuth> {
    pub(crate) client: Client<A>,
    pub(crate) data: T
}
