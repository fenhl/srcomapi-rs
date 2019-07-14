//! Contains the type representing a speedrun

use {
    std::{
        fmt,
        time::Duration
    },
    chrono::prelude::*,
    reqwest::Url,
    serde_derive::Deserialize,
    crate::{
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    /// The run has been verified by a leaderboard moderator.
    #[serde(rename_all = "kebab-case")]
    Verified {
        /// The ID of the user who verified the run. Can be `None` for old runs.
        examiner: Option<String>,
        /// The time when the run was verified. Can be `None` for old runs.
        verify_date: Option<DateTime<Utc>>
    },
    /// The run has been rejected by a leaderboard moderator.
    Rejected {
        /// The ID of the user who rejected the run. Can be `None` for old runs.
        examiner: Option<String>,
        /// The reason why the run was rejected, given by the examiner.
        reason: String
    }
}

impl RunStatus {
    /// The user who verified or rejected this run. Returns `Ok(None)` if the run has neither been verified nor rejected, or if it's unknown who did so.
    pub fn examiner(&self, client: &Client) -> Result<Option<User>> {
        Ok(match self {
            RunStatus::Verified { examiner: Some(id), .. }
            | RunStatus::Rejected { examiner: Some(id), .. } => Some(User::from_id(client, id)?),
            _ => None
        })
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

    /// The user who verified or rejected this run. Returns `Ok(None)` if the run has neither been verified nor rejected, of if it's unknown who did so.
    pub fn examiner(&self, client: &Client) -> Result<Option<User>> {
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
