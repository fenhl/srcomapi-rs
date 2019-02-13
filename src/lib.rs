//! A Rust client for the [speedrun.com REST API](https://github.com/speedruncomorg/api).

#![cfg_attr(test, deny(warnings))]
#![warn(trivial_casts)]
#![deny(missing_docs, unused, unused_qualifications)]
#![forbid(unused_import_braces)]

use wrapped_enum::wrapped_enum;

pub mod client;
pub mod model;
pub mod paginated;
pub(crate) mod util;

/// A collection of possible errors not simply forwarded from other libraries.
#[derive(Debug)]
pub enum OtherError {
    /// Returned by `Category::game` if the API didn't return a link with `"rel": "game"`.
    MissingGameRel
}

wrapped_enum! {
    /// An enum that contains all the different kinds of errors that can occur in the library.
    #[derive(Debug)]
    pub enum Error {
        #[allow(missing_docs)]
        InvalidHeaderValue(reqwest::header::InvalidHeaderValue),
        #[allow(missing_docs)]
        Other(OtherError),
        #[allow(missing_docs)]
        Reqwest(reqwest::Error)
    }
}

/// The library's result type.
pub type Result<T> = std::result::Result<T, Error>;
