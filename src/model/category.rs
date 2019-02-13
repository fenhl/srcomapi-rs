//! Categories are the different rulesets for speedruns

use std::{
    fmt,
    iter::FromIterator
};
use itertools::Itertools;
use serde_derive::Deserialize;
use crate::{
    OtherError,
    Result,
    client::{
        AnnotatedData,
        Client,
        Link,
        ResponseData
    },
    model::{
        game::Game,
        run::{
            Run,
            RunData
        },
        variable::{
            Filter,
            Variable
        }
    }
};

#[derive(Debug, Deserialize, Clone)]
struct Leaderboard {
    runs: Vec<LeaderboardEntry>
}

#[derive(Debug, Deserialize, Clone)]
struct LeaderboardEntry {
    place: usize,
    run: RunData
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum CategoryType {
    PerGame,
    PerLevel
}

/// The cached data for a category. This type is an implementation detail. You're probably looking for `Category` instead.
#[derive(Debug, Deserialize, Clone)]
pub struct CategoryData {
    id: String,
    links: Vec<Link>,
    name: String,
    #[serde(rename = "type")]
    cat_type: CategoryType
}

/// Categories are the different rulesets for speedruns.
pub type Category = AnnotatedData<CategoryData>;

impl Category {
    /// Returns the category with the given ID.
    pub fn from_id(client: &Client, id: impl fmt::Display) -> Result<Category> {
        Ok(client.annotate(
            client.get(format!("/categories/{}", id))
                .send()?
                .error_for_status()?
                .json::<ResponseData<_>>()?
                .data
        ))
    }

    /// Returns the game to which this category belongs.
    pub fn game(&self) -> Result<Game> {
        let (link,) = self.data.links.iter()
            .filter(|link| &link.rel == "game")
            .collect_tuple().ok_or(OtherError::MissingGameRel)?;
        Ok(self.client.annotate(
            self.client.get_abs(link.uri.clone())
                .send()?
                .error_for_status()?
                .json::<ResponseData<_>>()?
                .data
        ))
    }

    /// Returns this category's API ID.
    pub fn id(&self) -> &str {
        &self.data.id
    }

    /// Returns `true` if this is an IL (individual level) category.
    pub fn is_il(&self) -> bool {
        self.data.cat_type == CategoryType::PerLevel
    }

    /// Returns the leaderboard for this full-game category, i.e. all non-obsoleted runs.
    ///
    /// # Errors
    ///
    /// Will error if this is an IL category.
    pub fn leaderboard<C: FromIterator<Run>>(&self) -> Result<C> {
        self.leaderboard_filtered(&Filter::default())
    }

    /// Returns a leaderboard for this full-game category, filtered by the given variable/value pairs.
    ///
    /// # Errors
    ///
    /// Will error if this is an IL category.
    pub fn leaderboard_filtered<C: FromIterator<Run>>(&self, filter: &Filter) -> Result<C> {
        Ok(
            self.client.get(format!("/leaderboards/{}/category/{}", self.game()?.id(), self.data.id))
                .query(filter)
                .send()?
                .error_for_status()?
                .json::<ResponseData<Leaderboard>>()?
                .data
                .runs
                .into_iter()
                .map(|entry| self.client.annotate(entry.run))
                .collect()
        )
    }

    /// Returns all variables applicable to this category.
    pub fn variables<C: FromIterator<Variable>>(&self) -> Result<C> {
        self.client.get_annotated_collection(format!("/categories/{}/variables", self.data.id))
    }

    /// A convenience method returning the first place from this category's leaderboard, i.e. the current world record of the category.
    ///
    /// In case of a tie or if no run has been verified for this category, `Ok(None)` is returned.
    pub fn wr(&self) -> Result<Option<Run>> {
        let mut lb = self.client.get(format!("/leaderboards/{}/category/{}", self.game()?.id(), self.data.id))
            .send()?
            .error_for_status()?
            .json::<ResponseData<Leaderboard>>()?
            .data
            .runs;
        if lb.is_empty() || lb.len() > 1 && lb[1].place == 1 { return Ok(None); }
        Ok(Some(self.client.annotate(lb.remove(0).run)))
    }
}

/// Displays the category name.
impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.data.name.fmt(f)
    }
}
