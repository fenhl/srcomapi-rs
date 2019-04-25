//! Contains the type representing a speedrun

use std::{
    fmt,
    time::Duration
};
use chrono::prelude::*;
use reqwest::Url;
use serde_derive::Deserialize;
use crate::{
    OtherError,
    Result,
    client::{
        AnnotatedData,
        Client
    },
    model::user::User,
    util::{
        DurationDef,
        OptDurationDef
    }
};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "rel")]
enum RunnerData {
    User { id: String },
    Guest { name: String }
}

/// A player who participated in this run.
pub enum Runner {
    /// A registered user.
    User(User),
    /// A guest of whom only a name is documented.
    Guest(String)
}

impl Runner {
    fn new(client: &Client, data: &RunnerData) -> Result<Runner> {
        Ok(match *data {
            RunnerData::User { ref id } => { Runner::User(User::from_id(client, id)?) } //TODO
            RunnerData::Guest { ref name } => Runner::Guest(name.clone())
        })
    }
}

/// Displays the users's international username.
impl fmt::Display for Runner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Runner::User(ref user) => user.fmt(f),
            Runner::Guest(ref name) => name.fmt(f)
        }
    }
}

/// The duration of a run in the different documented timing methods.
#[derive(Debug, Deserialize, Clone)]
pub struct Times {
    /// The primary time counted for the leaderboard. This will be the same as one of the other times.
    #[serde(with = "DurationDef")]
    pub primary: Duration,
    /// The real duration of the run.
    #[serde(with = "OptDurationDef")]
    pub realtime: Option<Duration>,
    /// The duration of the run when subtracting load times.
    #[serde(with = "OptDurationDef")]
    pub realtime_noloads: Option<Duration>,
    /// The run time as measured by the game.
    #[serde(with = "OptDurationDef")]
    pub ingame: Option<Duration>
}

/// The submission status of a run (verified, rejected, or new).
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "status", rename_all = "kebab-case")]
pub enum RunStatus {
    /// The run has neither been verified nor rejected yet.
    New,
    #[serde(rename_all = "kebab-case")]
    /// The run has been verified by a leaderboard moderator.
    Verified {
        /// The ID of the user who verified the run.
        examiner: String,
        /// The time when the run was verified. Can be `None` for old runs.
        verify_date: Option<DateTime<Utc>>
    },
    /// The run has been rejected by a leaderboard moderator.
    Rejected {
        /// The ID of the user who rejected the run.
        examiner: String,
        /// The reason why the run was rejected, given by the examiner.
        reason: String
    }
}

impl RunStatus {
    /// The user who verified or rejected this run. Returns `OtherError::UnverifiedRun` if the run has neither been verified nor rejected.
    pub fn examiner(&self, client: &Client) -> Result<User> {
        match self {
            RunStatus::New => Err(OtherError::UnverifiedRun.into()),
            RunStatus::Verified { examiner, .. }
            | RunStatus::Rejected { examiner, .. } => User::from_id(client, examiner)
        }
    }
}

/// The cached data for a speedrun. This type is an implementation detail. You're probably looking for `Run` instead.
#[derive(Debug, Deserialize, Clone)]
pub struct RunData {
    date: Option<NaiveDate>,
    id: String,
    players: Vec<RunnerData>,
    status: RunStatus,
    submitted: Option<DateTime<Utc>>,
    times: Times,
    #[serde(with = "url_serde")]
    weblink: Url
}

/// The type representing a speedrun.
pub type Run = AnnotatedData<RunData>;

impl Run {
    /// Returns this run's API ID.
    pub fn id(&self) -> &str {
        &self.data.id
    }

    /// The date on which the run was played, if known. Submitted by the runner.
    pub fn date(&self) -> Option<NaiveDate> {
        self.data.date
    }

    /// The user who verified or rejected this run. Returns `OtherError::UnverifiedRun` if the run has neither been verified nor rejected.
    pub fn examiner(&self, client: &Client) -> Result<User> {
        self.status().examiner(client)
    }

    /// Returns the list of players who participated in this run.
    pub fn runners(&self) -> Result<Vec<Runner>> {
        self.data.players.iter()
            .map(|runner_data| Runner::new(&self.client, runner_data))
            .collect()
    }

    /// The current submission status of this run (verified, rejected, or new).
    pub fn status(&self) -> &RunStatus {
        &self.data.status
    }

    /// The time when the run was submitted to the leaderboard. Can be `None` for old runs.
    pub fn submitted(&self) -> Option<DateTime<Utc>> {
        self.data.submitted
    }

    /// Returns the duration of this run in the primary timing method used by the leaderboard.
    pub fn time(&self) -> Duration {
        self.data.times.primary
    }

    /// Returns the duration of this run in the different documented timing methods.
    pub fn times(&self) -> &Times {
        &self.data.times
    }

    /// Returns the URL to the run's page on speedrun.com.
    pub fn weblink(&self) -> &Url {
        &self.data.weblink
    }
}
