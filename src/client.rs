//! The `Client` type is the entry point to the API.

use std::{
    borrow::Borrow,
    collections::HashMap,
    fmt,
    fs::File,
    iter::FromIterator,
    marker::PhantomData,
    ops::{
        Range,
        RangeTo
    },
    path::PathBuf,
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
use rand::prelude::*;
use reqwest::{
    self,
    IntoUrl,
    Url
};
use serde::de::DeserializeOwned;
use serde_derive::{
    Deserialize,
    Serialize
};
use url_serde::Serde;
use crate::Result;

/// The maximum number requests allowed by the API within one `RATE_LIMIT_INTERVAL`. This number is made public for informational purposes only; the `Client` adheres to the rate limit automatically.
pub const RATE_LIMIT_NUM_REQUESTS: usize = 100;

/// The duration window used for rate limiting. This number is made public for informational purposes only; the `Client` adheres to the rate limit automatically.
pub const RATE_LIMIT_INTERVAL: Duration = Duration::from_secs(60);

static BASE_URL: &str = "https://www.speedrun.com/api/v1";

#[derive(Debug, Deserialize, Serialize)]
struct RequestInfo {
    timestamp: SystemTime,
    data: serde_json::Value
}

/// Helper trait implemented on the marker types `NoAuth` and `Auth`.
pub trait AuthType<'a> {
    /// Used to stor the API key in `Builder<Auth>`.
    type Info: 'a;
}

/// A marker type used as a type parameter on `Client` to indicate that the client is authenticated.
#[derive(Debug, Clone, Copy)]
pub enum Auth {}

impl<'a> AuthType<'a> for Auth {
    type Info = &'a str;
}

/// A marker type used as a type parameter on `Client` to indicate that the client is not authenticated. This is the default.
#[derive(Debug, Clone, Copy)]
pub enum NoAuth {}

impl<'a> AuthType<'a> for NoAuth {
    type Info = ();
}

/// The trait for parameters to `Builder::cache_timeout`.
pub trait IntoTimeout {
    /// Performs the conversion.
    ///
    /// See `Builder::cache_timeout` for details.
    fn into_timeout(self) -> Option<Range<Duration>>;
}

impl IntoTimeout for Duration {
    fn into_timeout(self) -> Option<Range<Duration>> {
        Some(self..self)
    }
}

impl IntoTimeout for Range<Duration> {
    fn into_timeout(self) -> Option<Range<Duration>> {
        Some(self)
    }
}

impl IntoTimeout for RangeTo<Duration> {
    fn into_timeout(self) -> Option<Range<Duration>> {
        Some(Duration::default()..self.end)
    }
}

impl IntoTimeout for () {
    fn into_timeout(self) -> Option<Range<Duration>> {
        None
    }
}

impl<T: IntoTimeout> IntoTimeout for Option<T> {
    fn into_timeout(self) -> Option<Range<Duration>> {
        self.and_then(T::into_timeout)
    }
}

fn timestamp_is_valid(timestamp: SystemTime, timeout: &Range<Duration>) -> bool {
    timestamp.elapsed().map(|elapsed|
        elapsed < timeout.start
        || elapsed < timeout.end
        && thread_rng().gen_bool((timeout.end - elapsed).as_secs() as f64 / (timeout.end - timeout.start).as_secs() as f64) //TODO use Duration::div_duration when stable
    ).unwrap_or_default()
}

/// A `Client` builder that allows configuring additional settings of the client.
#[derive(Debug)]
pub struct Builder<'a, A: AuthType<'a> = NoAuth> {
    user_agent: &'static str,
    api_key: A::Info,
    cache: HashMap<Url, RequestInfo>,
    cache_path: Option<PathBuf>,
    cache_timeout: Option<Range<Duration>>,
    num_tries: u8
}

impl<'a> Builder<'a, NoAuth> {
    /// Creates a new client builder with the given user agent and default values for the other options.
    ///
    /// For details on the user agent, see the `Client::new` docs.
    ///
    /// For details on the other configuration options, as well as their default values, see the docs on the respective methods.
    pub fn new(user_agent: &'static str) -> Builder {
        Builder {
            user_agent,
            api_key: (),
            cache: HashMap::default(),
            cache_path: None,
            cache_timeout: Some(RATE_LIMIT_INTERVAL..RATE_LIMIT_INTERVAL),
            num_tries: 1
        }
    }

    /// When used, the resulting client will authenticate as a user using the given API key.
    ///
    /// For details on obtaining a user's API key, see [the docs on authentication](https://github.com/speedruncomorg/api/blob/master/authentication.md).
    ///
    /// The default client is unauthenticated and cannot access API endpoints that require authentication. This library enforces that restriction on the type level.
    pub fn auth(self, api_key: &str) -> Builder<Auth> {
        Builder {
            user_agent: self.user_agent,
            api_key,
            cache: self.cache,
            cache_path: self.cache_path,
            cache_timeout: self.cache_timeout,
            num_tries: self.num_tries
        }
    }

    /// Builds and returns the configured client.
    ///
    /// # Errors
    ///
    /// This method fails if native TLS backend cannot be initialized.
    ///
    /// # Panics
    ///
    /// This method panics if the user agent contains invalid [header value characters](https://docs.rs/reqwest/*/reqwest/header/struct.HeaderValue.html#method.from_static).
    pub fn build(self) -> Result<Client<NoAuth>> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::USER_AGENT, reqwest::header::HeaderValue::from_static(self.user_agent));
        Ok(Client {
            cache: Cache::new(self.cache, self.cache_path, self.cache_timeout),
            num_tries: self.num_tries,
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()?,
            phantom: PhantomData
        })
    }
}

impl<'a> Builder<'a, Auth> {
    /// Builds and returns the configured client.
    ///
    /// # Errors
    ///
    /// This method fails if native TLS backend cannot be initialized or the API key contains invalid [header value characters](https://docs.rs/reqwest/*/reqwest/header/struct.HeaderValue.html#method.from_static).
    ///
    /// # Panics
    ///
    /// This method panics if the user agent contains invalid [header value characters](https://docs.rs/reqwest/*/reqwest/header/struct.HeaderValue.html#method.from_static).
    pub fn build(self) -> Result<Client<Auth>> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(reqwest::header::USER_AGENT, reqwest::header::HeaderValue::from_static(self.user_agent));
        headers.insert("X-API-Key", reqwest::header::HeaderValue::from_str(self.api_key)?);
        Ok(Client {
            cache: Cache::new(self.cache, self.cache_path, self.cache_timeout),
            num_tries: self.num_tries,
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()?,
            phantom: PhantomData
        })
    }
}

impl<'a, A: AuthType<'a>> Builder<'a, A> {
    /// Configures the duration for which a given API response will be cached.
    ///
    /// `None` means cache entries live forever and once a response for a given endpoint has been cached will be reused for the remainder of the client's lifetime.
    ///
    /// A range means that cache entries whose age lies within the range are randomly considered valid or invalid.
    ///
    /// The default value is the value of `RATE_LIMIT_INTERVAL`, i.e. the same as the rate limiting interval.
    pub fn cache_timeout(self, cache_timeout: impl IntoTimeout) -> Builder<'a, A> {
        Builder {
            cache_timeout: cache_timeout.into_timeout(),
            ..self
        }
    }

    /// Initializes the cache for API responses from disk.
    ///
    /// Cache entries older than the currently configured `cache_timeout` are discarded when read, so `cache_timeout` must be called *before* this method to work as expected.
    ///
    /// # Errors
    ///
    /// If an I/O error occurs, or if the file is not a valid cache.
    pub fn disk_cache(self, cache_path: PathBuf) -> Result<Builder<'a, A>> {
        let mut cache = serde_json::from_reader::<_, HashMap<Serde<Url>, RequestInfo>>(File::open(&cache_path)?)?;
        if let Some(ref timeout) = self.cache_timeout {
            cache.retain(|_, req_info| timestamp_is_valid(req_info.timestamp, timeout));
        }
        Ok(Builder {
            cache: cache.into_iter().map(|(url, info)| (url.into_inner(), info)).collect(),
            cache_path: Some(cache_path),
            ..self
        })
    }

    /// Configures the number of times each request is attempted before a server or network error is returned.
    ///
    /// Client errors are always returned immediately and not retried.
    ///
    /// The default value is 1, meaning server errors are also returned immediately.
    ///
    /// # Panics
    ///
    /// When `0` is passed.
    pub fn num_tries(self, num_tries: u8) -> Builder<'a, A> {
        if num_tries == 0 { panic!("0 passed to srcomapi::client::Builder::num_tries"); }
        Builder { num_tries, ..self }
    }
}

#[derive(Debug)]
struct Cache {
    data: HashMap<Url, RequestInfo>,
    path: Option<PathBuf>,
    timeout: Option<Range<Duration>>,
    changes: u8
}

impl Cache {
    fn new(data: HashMap<Url, RequestInfo>, path: Option<PathBuf>, timeout: Option<Range<Duration>>) -> Arc<RwLock<Cache>> {
        Arc::new(RwLock::new(Cache {
            data, path, timeout,
            changes: 0
        }))
    }

    fn get(&self, url: &Url) -> Option<serde_json::Value> {
        if let Some(cache_entry) = self.data.get(url) {
            if self.timeout.as_ref().map_or(true, |timeout| timestamp_is_valid(cache_entry.timestamp, timeout)) {
                return Some(cache_entry.data.clone());
            }
        }
        None
    }

    fn insert(&mut self, url: Url, info: RequestInfo) {
        self.data.insert(url, info);
        self.changes += 1;
        if self.changes >= 16 {
            if let Ok(()) = self.persist() {
                self.changes = 0;
            }
        }
    }

    fn persist(&self) -> Result<()> {
        if let Some(ref path) = self.path {
            serde_json::to_writer(File::create(path)?, &self.data.iter().map(|(url, info)| (Serde(url.clone()), info)).collect::<HashMap<_, _>>())?;
        }
        Ok(())
    }

    fn rate_limited(&self) -> Result<Option<Duration>> {
        let recent_request_times = self.data.values().map(|cache_entry| cache_entry.timestamp).filter(|timestamp| timestamp.elapsed().map(|elapsed| elapsed < RATE_LIMIT_INTERVAL).unwrap_or(true)).collect::<Vec<_>>();
        if recent_request_times.len() >= RATE_LIMIT_NUM_REQUESTS {
            let elapsed = recent_request_times.iter().min().unwrap().elapsed()?;
            if elapsed < RATE_LIMIT_INTERVAL {
                return Ok(Some(RATE_LIMIT_INTERVAL - elapsed));
            }
        }
        Ok(None)
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        let _ = self.persist();
    }
}

/// The entry point to the API.
///
/// The client automatically inserts pauses between requests if necessary according to the API's [rate limits](https://github.com/speedruncomorg/api/blob/master/throttling.md). However, this only works if your application uses the same `Client` for all API requests. If you use multiple `Client`s, you risk getting HTTP `420` errors due to rate limiting.
#[derive(Debug, Clone)]
pub struct Client<A = NoAuth> {
    cache: Arc<RwLock<Cache>>,
    num_tries: u8,
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
    /// For additional configuration options, use the `Builder` type instead.
    ///
    /// # Errors
    ///
    /// This method fails if native TLS backend cannot be initialized.
    ///
    /// # Panics
    ///
    /// This method panics if the user agent contains invalid [header value characters](https://docs.rs/reqwest/*/reqwest/header/struct.HeaderValue.html#method.from_static).
    pub fn new(user_agent: &'static str) -> Result<Client> {
        Builder::new(user_agent).build()
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
    /// For additional configuration options, use the `Builder` type instead.
    ///
    /// # Errors
    ///
    /// This method fails if native TLS backend cannot be initialized or the API key contains invalid [header value characters](https://docs.rs/reqwest/*/reqwest/header/struct.HeaderValue.html#method.from_static).
    ///
    /// # Panics
    ///
    /// This method panics if the user agent contains invalid [header value characters](https://docs.rs/reqwest/*/reqwest/header/struct.HeaderValue.html#method.from_static).
    pub fn new(user_agent: &'static str, api_key: &str) -> Result<Client<Auth>> {
        Builder::new(user_agent).auth(api_key).build()
    }
}

impl<A> Client<A> {
    pub(crate) fn get_raw<U: IntoUrl, K: AsRef<str>, V: AsRef<str>, Q: IntoIterator, T: DeserializeOwned>(&self, url: U, query: Q) -> Result<T>
    where Q::Item: Borrow<(K, V)> {
        let mut url = url.into_url()?;
        url.query_pairs_mut().extend_pairs(query);
        Ok(loop {
            // check cache
            if let Some(cache_entry) = self.cache.read().expect("cache lock poisoned").get(&url) {
                break serde_json::from_value(cache_entry)?;
            }
            // wait for rate limit
            let mut cache = self.cache.write().expect("cache lock poisoned");
            if let Some(rate_limit_timeout) = cache.rate_limited()? {
                drop(cache);
                thread::sleep(rate_limit_timeout);
                continue;
            }
            // send request
            let mut response_data = self.client.get(url.clone())
                .send()
                .and_then(|resp| resp.error_for_status())
                .and_then(|mut resp| resp.json::<serde_json::Value>());
            for _ in 1..self.num_tries {
                match response_data {
                    Ok(_) => { break; }
                    Err(e) => if e.is_client_error() || e.is_serialization() { return Err(e.into()); } // return client errors immediately
                }
                response_data = self.client.get(url.clone())
                    .send()
                    .and_then(|resp| resp.error_for_status())
                    .and_then(|mut resp| resp.json::<serde_json::Value>());
            }
            let response_data = response_data?;
            // insert response into cache
            cache.insert(url, RequestInfo {
                timestamp: SystemTime::now(),
                data: response_data.clone()
            });
            // return response
            break serde_json::from_value(response_data)?;
        })
    }

    pub(crate) fn get<U: fmt::Display, T: DeserializeOwned>(&self, url: U) -> Result<T> {
        self.get_abs(&format!("{}{}", BASE_URL, url))
    }

    pub(crate) fn get_abs<U: IntoUrl, T: DeserializeOwned>(&self, url: U) -> Result<T> {
        self.get_abs_query(url, &Vec::<(String, String)>::default())
    }

    pub(crate) fn get_query<U: fmt::Display, K: AsRef<str>, V: AsRef<str>, Q: IntoIterator, T: DeserializeOwned>(&self, url: U, query: Q) -> Result<T>
    where Q::Item: Borrow<(K, V)> {
        self.get_abs_query(&format!("{}{}", BASE_URL, url), query)
    }

    pub(crate) fn get_abs_query<U: IntoUrl, K: AsRef<str>, V: AsRef<str>, Q: IntoIterator, T: DeserializeOwned>(&self, url: U, query: Q) -> Result<T>
    where Q::Item: Borrow<(K, V)> {
        Ok(self.get_raw::<_, _, _, _, ResponseData<_>>(url, query)?.data)
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
            self.get::<_, Vec<_>>(url)?
                .into_iter()
                .map(|data| self.annotate(data))
                .collect() //TODO get rid of this (lifetime issues)
        )
    }
}

impl From<Client<Auth>> for Client<NoAuth> {
    fn from(auth_client: Client<Auth>) -> Client<NoAuth> {
        Client {
            cache: auth_client.cache,
            num_tries: auth_client.num_tries,
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
struct ResponseData<T> {
    data: T
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct Link {
    pub(crate) rel: String,
    #[serde(with = "url_serde")]
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
