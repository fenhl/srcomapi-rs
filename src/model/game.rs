//! Games are the things users do speedruns in

use std::fmt;
use super::super::{
    Result,
    client::{
        AnnotatedData,
        Client,
        ResponseData
    },
    model::{
        category::Category,
        paginated::PaginatedList
    },
    util::UrlDef
};
use reqwest::Url;

pub(crate) static LIST_URL: &'static str = "/games?_bulk=yes";

/// The different names registered for a game.
#[derive(Debug, Deserialize)]
pub struct Names {
    /// The game's international, or main, name.
    pub international: String,
    /// The game's Japanese name, if registered.
    pub japanese: Option<String>,
    /// The game's name on [twitch.tv](https://www.twitch.tv/), if registered.
    pub twitch: Option<String>
}

/// The cached data for a game. This type is an implementation detail. You're probably looking for `Game` instead.
#[derive(Debug, Deserialize)]
pub struct GameData {
    id: String,
    abbreviation: String,
    names: Names,
    #[serde(with = "UrlDef")]
    weblink: Url
}

/// Games are the things users do speedruns in.
pub type Game = AnnotatedData<GameData>;

impl Game {
    /// Returns a paginated list of all games on speedrun.com.
    pub fn list(client: impl Into<Client>) -> PaginatedList<GameData> {
        let mut list = PaginatedList::new(client.into(), LIST_URL.into());
        list.set_page_size(1000);
        list
    }

    /// Returns the game with the given ID or abbreviation.
    pub fn from_id(client: &Client, id: impl fmt::Display) -> Result<Game> {
        Ok(client.annotate(
            client.get(format!("/games/{}", id))
                .send()?
                .error_for_status()?
                .json::<ResponseData<_>>()?
                .data
        ))
    }

    /// Returns all speedrun categories defined for the game.
    pub fn categories(&self) -> Result<Vec<Category>> {
        Ok(
            self.client.get(format!("/games/{}/categories", self.data.id))
                .send()?
                .error_for_status()?
                .json::<ResponseData<Vec<_>>>()?
                .data
                .into_iter()
                .map(|cat_data| self.client.annotate(cat_data))
                .collect()
        )
    }

    /// Returns this game's API ID.
    pub fn id(&self) -> &str {
        &self.data.id
    }

    /// Returns the different names registered for this game.
    pub fn names(&self) -> &Names {
        &self.data.names
    }

    /// Returns the link to this game's page intended for humans.
    pub fn weblink(&self) -> &Url {
        &self.data.weblink
    }
}

/// Displays the game's English name.
impl fmt::Display for Game {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.data.names.international.fmt(f)
    }
}
