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
        Link
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
pub(crate) struct Leaderboard {
    pub(crate) runs: Vec<LeaderboardEntry>
}

#[derive(Debug, Deserialize, Clone)]
pub(crate) struct LeaderboardEntry {
    pub(crate) place: usize,
    pub(crate) run: RunData
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
            client.get(format!("/categories/{}", id))?
        ))
    }

    /// Returns the game to which this category belongs.
    pub fn game(&self) -> Result<Game> {
        let (link,) = self.data.links.iter()
            .filter(|link| &link.rel == "game")
            .collect_tuple().ok_or(OtherError::MissingGameRel)?;
        Ok(self.client.annotate(
            self.client.get_abs(link.uri.clone())?
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

    /// Returns all variables applicable to this category.
    pub fn variables<C: FromIterator<Variable>>(&self) -> Result<C> {
        self.client.get_annotated_collection(format!("/categories/{}/variables", self.id()))
    }
}

/// This trait is implemented on types for which leaderboards are available, namely:
///
/// * `&Category` (full-game leaderboards), and
/// * `(&Level, &Category)` (individual-level leaderboards).
///
/// It provides methods to access these leaderboards.
pub trait ToLeaderboard: Sized {
    /// Returns a leaderboard for this category, filtered by the given variable/value pairs.
    fn filtered_leaderboard<C: FromIterator<Run>>(self, filter: &Filter) -> Result<C>;

    /// A convenience method returning the first place from a filtered version of this category's leaderboard.
    fn filtered_wr(self, filter: &Filter) -> Result<Option<Run>>;

    /// Returns true if the world record for this category and the given filter is tied.
    fn filtered_wr_is_tied(self, filter: &Filter) -> Result<bool>;

    /// Returns the leaderboard for this category, i.e. all non-obsoleted runs.
    fn leaderboard<C: FromIterator<Run>>(self) -> Result<C> {
        self.filtered_leaderboard(&Filter::default())
    }

    /// A convenience method returning the first place from this category's leaderboard, i.e. the current world record of the category.
    ///
    /// If the world record is tied, this method returns whichever run the API lists first.
    ///
    /// If no run has been verified for this category, `Ok(None)` is returned.
    fn wr(self) -> Result<Option<Run>> {
        self.filtered_wr(&Filter::default())
    }

    /// Returns true if the world record for this category is tied.
    fn wr_is_tied(self) -> Result<bool> {
        self.filtered_wr_is_tied(&Filter::default())
    }
}

/// Displays the category name.
impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.data.name.fmt(f)
    }
}

impl ToLeaderboard for &Category {
    /// Returns a leaderboard for this full-game category, filtered by the given variable/value pairs.
    ///
    /// # Errors
    ///
    /// Will error if this is an IL category.
    fn filtered_leaderboard<C: FromIterator<Run>>(self, filter: &Filter) -> Result<C> {
        Ok(
            self.client.get_query::<_, _, Leaderboard>(format!("/leaderboards/{}/category/{}", self.game()?.id(), self.id()), filter)?
                .runs
                .into_iter()
                .map(|entry| self.client.annotate(entry.run))
                .collect()
        )
    }

    /// A convenience method returning the first place from a filtered version of this category's leaderboard.
    ///
    /// If the world record is tied, this method returns whichever run the API lists first.
    ///
    /// If no run has been verified for the given filter, `Ok(None)` is returned.
    fn filtered_wr(self, filter: &Filter) -> Result<Option<Run>> {
        let mut lb = self.client.get_query::<_, _, Leaderboard>(format!("/leaderboards/{}/category/{}", self.game()?.id(), self.id()), filter)?
            .runs;
        if lb.is_empty() { return Ok(None); }
        Ok(Some(self.client.annotate(lb.remove(0).run)))
    }

    /// Returns true if the world record for this category and the given filter is tied.
    fn filtered_wr_is_tied(self, filter: &Filter) -> Result<bool> {
        let lb = self.client.get_query::<_, _, Leaderboard>(format!("/leaderboards/{}/category/{}", self.game()?.id(), self.id()), filter)?
            .runs;
        Ok(lb.len() > 1 && lb[1].place == 1)
    }
}
