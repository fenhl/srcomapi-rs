//! A data structure for working with [paginated](https://github.com/speedruncomorg/api/blob/master/version1/pagination.md) endpoints

use std::{
    iter::FusedIterator,
    vec
};
use serde::de::DeserializeOwned;
use serde_derive::Deserialize;
use crate::{
    Result,
    client::{
        AnnotatedData,
        Client
    },
    model::game
};

#[derive(Debug, Deserialize)]
struct PaginationInfo {
    max: u16,
    size: u16
}

#[derive(Debug, Deserialize)]
struct PaginatedResult<T> {
    data: Vec<T>,
    pagination: PaginationInfo
}

/// This iterator represents a list of items returned by the API in chunks of pages.
///
/// # Errors
///
/// All requests are performed lazily: accessing an item that's on a page which has not yet been loaded will cause an API request for that page. Accordingly, most iterator methods can return request errors.
#[derive(Debug)]
pub struct PaginatedList<T: DeserializeOwned> {
    client: Client,
    prefix_len: usize,
    cached_prefix: vec::IntoIter<T>,
    end_seen: bool,
    page_size: u16,
    uri: String
}

impl<T: DeserializeOwned> PaginatedList<T> {
    pub(crate) fn new(client: Client, uri: String) -> PaginatedList<T> {
        PaginatedList {
            client, uri,
            prefix_len: 0,
            cached_prefix: Vec::default().into_iter(),
            end_seen: false,
            page_size: 20
        }
    }

    /// Returns the number of elements per request.
    ///
    /// For most lists, this will be a number in `1..=200`. However, the list of all games can have a page size of up to 1000.
    pub fn page_size(&self) -> u16 {
        self.page_size
    }

    /// Modifies the page size used for future requests.
    ///
    /// # Panics
    ///
    /// For the list of all games, panics if the given page size is not in `1..=1000`. For all other lists, panics if the given page size is not in `1..=200`.
    pub fn set_page_size(&mut self, page_size: u16) {
        if &self.uri == game::LIST_URL {
            if page_size < 1 || page_size > 1000 {
                panic!("argument for PaginatedList::set_page_size should be in 1..=1000, was {:?}", page_size);
            }
        } else {
            if page_size < 1 || page_size > 200 {
                panic!("argument for PaginatedList::set_page_size should be in 1..=200, was {:?}", page_size);
            }
        }
        self.page_size = page_size.into();
    }
}

impl<T: DeserializeOwned> Iterator for PaginatedList<T> {
    type Item = Result<AnnotatedData<T>>;

    fn next(&mut self) -> Option<Result<AnnotatedData<T>>> {
        // first, try to take the next item from the cached prefix or page, this works because vec::IntoIter implements FusedIterator
        if let Some(next_inner) = self.cached_prefix.next() {
            return Some(Ok(self.client.annotate(next_inner)));
        }
        // if the cache is empty and we've seen the end, we're done
        if self.end_seen { return None; }
        // if the cache is empty and we haven't seen the end, download and cache the next page
        let resp = match self.client.get(&self.uri)
            .query(&[("offset", self.prefix_len)])
            .query(&[("max", self.page_size)])
            .send()
        {
            Ok(resp) => resp,
            Err(e) => { return Some(Err(e.into())); }
        };
        let mut resp = match resp.error_for_status() {
            Ok(resp) => resp,
            Err(e) => { return Some(Err(e.into())); }
        };
        let PaginatedResult { data, pagination } = match resp.json() {
            Ok(j) => j,
            Err(e) => { return Some(Err(e.into())); }
        };
        assert_eq!(usize::from(pagination.size), data.len());
        if pagination.size < pagination.max { self.end_seen = true; }
        self.cached_prefix = data.into_iter();
        self.prefix_len += usize::from(pagination.size);
        // take the first element from the new page. If it's empty, we're done
        self.cached_prefix.next().map(|item| Ok(self.client.annotate(item)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.cached_prefix.len();
        (len, if self.end_seen { Some(len) } else { None })
    }
}

impl<T: DeserializeOwned> FusedIterator for PaginatedList<T> {}
