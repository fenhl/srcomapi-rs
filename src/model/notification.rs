//! Notifications are system-generated messages sent to users when certain events concerning them happen on the site, like somebody liking a post or a run being verified

use std::{
    fmt,
    iter::FromIterator
};
use chrono::prelude::*;
use super::super::{
    Result,
    client::{
        AnnotatedData,
        Auth,
        Client
    }
};

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
}

/// Displays the notification's text.
impl fmt::Display for Notification {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.data.text.fmt(f)
    }
}
