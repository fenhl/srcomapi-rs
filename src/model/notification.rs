//! Notifications are system-generated messages sent to users when certain events concerning them happen on the site, like somebody liking a post or a run being verified

use std::{
    fmt,
    iter::FromIterator
};
use chrono::prelude::*;
use reqwest::Url;
use serde_derive::Deserialize;
use crate::{
    Result,
    client::{
        AnnotatedData,
        Auth,
        Client
    },
    util::UrlDef
};

/// The kind of link contained in a notification. Returned by `Notification::webllink_rel`.
#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Rel {
    /// someone liked the forum post
    Post,
    /// a run is linked from the notification
    Run,
    /// when a game request was approved/denied
    Game,
    /// when a guide was updated
    Guide
}

#[derive(Debug, Deserialize, Clone)]
struct Item {
    rel: Rel,
    #[serde(with = "UrlDef")]
    uri: Url
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum ReadStatus {
    Read,
    Unread
}

impl From<ReadStatus> for bool {
    fn from(status: ReadStatus) -> bool {
        match status {
            ReadStatus::Read => true,
            ReadStatus::Unread => false
        }
    }
}

/// The cached data for a notification. This type is an implementation detail. You're probably looking for `Notification` instead.
#[derive(Debug, Deserialize, Clone)]
pub struct NotificationData {
    id: String,
    created: DateTime<Utc>,
    item: Item,
    status: ReadStatus,
    text: String
}

/// Notifications are system-generated messages sent to users when certain events concerning them happen on the site, like somebody liking a post or a run being verified.
pub type Notification = AnnotatedData<NotificationData, Auth>;

impl Notification {
    /// Returns a paginated list of all games on speedrun.com.
    pub fn list<C: FromIterator<Notification>>(client: &Client<Auth>) -> Result<C> {
        client.get_annotated_collection("/notifications")
    }

    /// Returns this notification's API ID.
    pub fn id(&self) -> &str {
        &self.data.id
    }

    /// Returns the timestamp when this notification was created.
    pub fn created(&self) -> &DateTime<Utc> {
        &self.data.created
    }

    /// Returns `true` if this notification is marked as read.
    pub fn read(&self) -> bool {
        self.data.status.into()
    }

    /// Returns the link contained in this notification. May point to the homepage.
    pub fn weblink(&self) -> &Url {
        &self.data.item.uri
    }

    /// The kind of item the `weblink` points at.
    pub fn weblink_rel(&self) -> Rel {
        self.data.item.rel
    }
}

/// Displays the notification's text.
impl fmt::Display for Notification {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.data.text.fmt(f)
    }
}
