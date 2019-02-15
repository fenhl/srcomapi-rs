//! Users are the individuals who have registered an account on speedrun.com

use std::fmt;
use chrono::prelude::*;
use serde_derive::Deserialize;
use crate::{
    Result,
    client::{
        AnnotatedData,
        Client
    },
    paginated::PaginatedList
};

/// The different names a user has registered.
#[derive(Debug, Deserialize, Clone)]
pub struct Names {
    /// The user's international, or main, username.
    pub international: String,
    /// The user's Japanese name, if registered.
    pub japanese: Option<String>,
}

/// The cached data for a user. This type is an implementation detail. You're probably looking for `User` instead.
#[derive(Debug, Deserialize, Clone)]
pub struct UserData {
    id: String,
    names: Names,
    signup: Option<DateTime<Utc>>
}

/// Users are the individuals who have registered an account on speedrun.com.
pub type User = AnnotatedData<UserData>;

impl User {
    /// Returns a paginated list of all games on speedrun.com.
    pub fn list(client: impl Into<Client>) -> PaginatedList<UserData> {
        PaginatedList::new(client.into(), "/users".into())
    }

    /// Returns the user with the given ID or username.
    pub fn from_id(client: &Client, id: impl fmt::Display) -> Result<User> {
        Ok(client.annotate(
            client.get(format!("/users/{}", id))?
        ))
    }

    /// Returns the timestamp when this user account was created. `None` for old user accounts.
    pub fn signup(&self) -> &Option<DateTime<Utc>> {
        &self.data.signup
    }
}

/// Displays the users's international username.
impl fmt::Display for User {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.data.names.international.fmt(f)
    }
}
