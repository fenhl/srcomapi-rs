//! The `Client` type is the entry point to the API.

use std::{
    collections::VecDeque,
    fmt,
    iter::FromIterator,
    marker::PhantomData,
    sync::{
        Arc,
        RwLock
    },
    thread,
    time::{
        Duration,
        SystemTime
    }
};
use reqwest::{
    self,
    IntoUrl,
    RequestBuilder,
    Url
};
use serde::de::DeserializeOwned;
use serde_derive::Deserialize;
use crate::{
    Result,
    util::UrlDef
};

const RATE_LIMIT_NUM_REQUESTS: usize = 100;
const RATE_LIMIT_INTERVAL: Duration = Duration::from_secs(60);
static BASE_URL: &str = "https://www.speedrun.com/api/v1";

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
    request_timestamps: Arc<RwLock<VecDeque<SystemTime>>>,
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
            request_timestamps: Arc::new(RwLock::new(VecDeque::with_capacity(RATE_LIMIT_NUM_REQUESTS))),
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
            request_timestamps: Arc::new(RwLock::new(VecDeque::with_capacity(RATE_LIMIT_NUM_REQUESTS))),
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()?,
            phantom: PhantomData
        })
    }
}

impl<A> Client<A> {
    pub(crate) fn get(&self, url: impl fmt::Display) -> Result<RequestBuilder> {
        self.get_abs(&format!("{}{}", BASE_URL, url))
    }

    pub(crate) fn get_abs(&self, url: impl IntoUrl) -> Result<RequestBuilder> {
        // wait for rate limit
        'rate_limit: loop {
            let timestamps = self.request_timestamps.read().expect("request timestamps lock poisoned");
            if timestamps.len() >= RATE_LIMIT_NUM_REQUESTS {
                let elapsed = timestamps.front().unwrap().elapsed()?;
                if elapsed < RATE_LIMIT_INTERVAL {
                    drop(timestamps);
                    thread::sleep(RATE_LIMIT_INTERVAL - elapsed);
                }
            }
            let mut timestamps = self.request_timestamps.write().expect("request timestamps lock poisoned");
            while timestamps.len() >= RATE_LIMIT_NUM_REQUESTS {
                if timestamps.front().unwrap().elapsed()? < RATE_LIMIT_INTERVAL { continue 'rate_limit; }
                timestamps.pop_front();
            }
            // record new request time
            timestamps.push_back(SystemTime::now());
            // start request builder
            break Ok(self.client.get(url));
        }
    }
}

impl<A: Clone> Client<A> {
    pub(crate) fn annotate<T>(&self, data: T) -> AnnotatedData<T, A> {
        AnnotatedData {
            data,
            client: self.clone()
        }
    }

    pub(crate) fn get_annotated_collection<T: DeserializeOwned, C: FromIterator<AnnotatedData<T, A>>>(&self, url: impl fmt::Display) -> Result<C> {
        Ok(
            self.get(url)?
                .send()?
                .error_for_status()?
                .json::<ResponseData<Vec<_>>>()?
                .data
                .into_iter()
                .map(|data| self.annotate(data))
                .collect() //TODO get rid of this (lifetime issues)
        )
    }
}

impl From<Client<Auth>> for Client<NoAuth> {
    fn from(auth_client: Client<Auth>) -> Client<NoAuth> {
        Client {
            request_timestamps: auth_client.request_timestamps,
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

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct ResponseData<T> {
    pub(crate) data: T
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Link {
    pub(crate) rel: String,
    #[serde(with = "UrlDef")]
    pub(crate) uri: Url
}

/// This type is an implementation detail.
///
/// It is a helper type which includes data of some sort, as well as a copy of the client to make further API requests. Most API methods are defined on `AnnotatedData<T>` instances for some concrete `T`.
#[derive(Debug, Clone)]
pub struct AnnotatedData<T, A = NoAuth> {
    pub(crate) client: Client<A>,
    pub(crate) data: T
}

impl<T> From<AnnotatedData<T, Auth>> for AnnotatedData<T, NoAuth> {
    fn from(annotated_data: AnnotatedData<T, Auth>) -> AnnotatedData<T> {
        AnnotatedData {
            client: annotated_data.client.into(),
            data: annotated_data.data
        }
    }
}
