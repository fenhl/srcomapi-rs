//! A Rust client for the [speedrun.com REST API](https://github.com/speedruncomorg/api).

#![deny(missing_docs, rust_2018_idioms, unused, unused_import_braces, unused_qualifications, warnings)]

use {
    std::{
        io,
        time::SystemTimeError
    },
    derive_more::From
};

pub mod client;
pub mod model;
pub mod paginated;
pub(crate) mod util;

/// An enum that contains all the different kinds of errors that can occur in the library.
#[derive(Debug, From)]
#[allow(missing_docs)]
pub enum Error {
    InvalidHeaderValue(reqwest::header::InvalidHeaderValue),
    Io(io::Error),
    /// Returned by `Category::game` if the API didn't return a link with `"rel": "game"`.
    MissingGameRel,
    Reqwest(reqwest::Error),
    SerDe(serde_json::Error),
    SystemTime(SystemTimeError)
}

/// The library's result type.
pub type Result<T> = std::result::Result<T, Error>;
