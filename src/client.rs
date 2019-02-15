//! The `Client` type is the entry point to the API.

use std::{
    collections::HashMap,
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
    Url
};
use serde::{
    Serialize,
    de::DeserializeOwned
};
use serde_derive::Deserialize;
use crate::{
    Result,
    util::UrlDef
};

const RATE_LIMIT_NUM_REQUESTS: usize = 100;
const RATE_LIMIT_INTERVAL: Duration = Duration::from_secs(60);
static BASE_URL: &str = "https://www.speedrun.com/api/v1";

#[derive(Debug)]
struct RequestInfo {
    timestamp: SystemTime,
    data: serde_json::Value
}

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
    recent_requests: Arc<RwLock<HashMap<Url, RequestInfo>>>,
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
            recent_requests: Arc::new(RwLock::new(HashMap::default())),
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
            recent_requests: Arc::new(RwLock::new(HashMap::default())),
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()?,
            phantom: PhantomData
        })
    }
}

impl<A> Client<A> {
    pub(crate) fn get_raw<U: IntoUrl, Q: Serialize + ?Sized, T: DeserializeOwned>(&self, url: U, query: &Q) -> Result<T> {
        let url = url.into_url()?;
        Ok('rate_limit: loop {
            {
                // check cache
                let cache = self.recent_requests.read().expect("recent requests lock poisoned");
                if let Some(cache_entry) = cache.get(&url) {
                    return Ok(serde_json::from_value(cache_entry.data.clone())?);
                }
                // wait for rate limit
                if cache.len() >= RATE_LIMIT_NUM_REQUESTS {
                    let elapsed = cache.values().min_by_key(|cache_entry| cache_entry.timestamp).unwrap().timestamp.elapsed()?;
                    if elapsed < RATE_LIMIT_INTERVAL {
                        drop(cache);
                        thread::sleep(RATE_LIMIT_INTERVAL - elapsed);
                    }
                }
            }
            let mut cache = self.recent_requests.write().expect("recent requests lock poisoned");
            while cache.len() >= RATE_LIMIT_NUM_REQUESTS {
                if cache.values().min_by_key(|cache_entry| cache_entry.timestamp).unwrap().timestamp.elapsed()? < RATE_LIMIT_INTERVAL {
                    continue 'rate_limit;
                }
                let oldest_url = cache.iter().min_by_key(|(_, cache_entry)| cache_entry.timestamp).unwrap().0.clone();
                cache.remove(&oldest_url);
            }
            // send request
            let mut response = self.client.get(url)
                .query(query)
                .send()?
                .error_for_status()?;
            let response_data = response.json::<serde_json::Value>()?;
            // insert response into cache
            cache.insert(response.url().clone(), RequestInfo {
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

    pub(crate) fn get_abs<T: DeserializeOwned>(&self, url: impl IntoUrl) -> Result<T> {
        self.get_abs_query(url, &Vec::<(String, String)>::default())
    }

    pub(crate) fn get_query<U: fmt::Display, Q: Serialize + ?Sized, T: DeserializeOwned>(&self, url: U, query: &Q) -> Result<T> {
        self.get_abs_query(&format!("{}{}", BASE_URL, url), query)
    }

    pub(crate) fn get_abs_query<Q: Serialize + ?Sized, T: DeserializeOwned>(&self, url: impl IntoUrl, query: &Q) -> Result<T> {
        Ok(self.get_raw::<_, _, ResponseData<_>>(url, query)?.data)
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
            recent_requests: auth_client.recent_requests,
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
