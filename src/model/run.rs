//! Contains the type representing a speedrun

use std::{
    fmt,
    time::Duration
};
use reqwest::Url;
use super::super::{
    Result,
    client::{
        AnnotatedData,
        Client
    },
    model::user::User,
    util::{
        DurationDef,
        OptDurationDef,
        UrlDef
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

/// The cached data for a speedrun. This type is an implementation detail. You're probably looking for `Run` instead.
#[derive(Debug, Deserialize, Clone)]
pub struct RunData {
    id: String,
    players: Vec<RunnerData>,
    times: Times,
    #[serde(with = "UrlDef")]
    weblink: Url
}

/// The type representing a speedrun.
pub type Run = AnnotatedData<RunData>;

impl Run {
    /// Returns this run's API ID.
    pub fn id(&self) -> &str {
        &self.data.id
    }

    /// Returns the list of players who participated in this run.
    pub fn runners(&self) -> Result<Vec<Runner>> {
        self.data.players.iter()
            .map(|runner_data| Runner::new(&self.client, runner_data))
            .collect()
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
