//! Levels are the stages/worlds/maps within a game

use {
    std::{
        fmt,
        iter::FromIterator
    },
    itertools::Itertools,
    serde_derive::Deserialize,
    crate::{
        OtherError,
        Result,
        client::{
            AnnotatedData,
            Client,
            Link
        },
        model::{
            category::{
                Category,
                Leaderboard,
                ToLeaderboard
            },
            game::Game,
            run::Run,
            variable::Filter
        }
    }
};

/// The cached data for a level. This type is an implementation detail. You're probably looking for `Level` instead.
#[derive(Debug, Deserialize, Clone)]
pub struct LevelData {
    id: String,
    links: Vec<Link>,
    name: String
}

/// Levels are the stages/worlds/maps within a game.
pub type Level = AnnotatedData<LevelData>;

impl Level {
    /// Returns the level with the given ID.
    pub fn from_id(client: &Client, id: impl fmt::Display) -> Result<Level> {
        Ok(client.annotate(
            client.get(format!("/levels/{}", id))?
        ))
    }

    /// Returns the game to which this level belongs.
    pub fn game(&self) -> Result<Game> {
        let (link,) = self.data.links.iter()
            .filter(|link| &link.rel == "game")
            .collect_tuple().ok_or(OtherError::MissingGameRel)?;
        Ok(self.client.annotate(
            self.client.get_abs(link.uri.clone())?
        ))
    }

    /// Returns this level's API ID.
    pub fn id(&self) -> &str {
        &self.data.id
    }
}

/// Displays the level name.
impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.data.name.fmt(f)
    }
}

impl ToLeaderboard for (&Level, &Category) {
    /// Returns a leaderboard for this IL category, filtered by the given variable/value pairs.
    ///
    /// # Errors
    ///
    /// Will error if the category is a full-game category.
    fn filtered_leaderboard<C: FromIterator<Run>>(self, filter: &Filter) -> Result<C> {
        let (level, category) = self;
        Ok(
            level.client.get_query::<_, _, _, _, Leaderboard>(format!("/leaderboards/{}/level/{}/{}", level.game()?.id(), level.id(), category.id()), filter)?
                .runs
                .into_iter()
                .map(|entry| level.client.annotate(entry.run))
                .collect()
        )
    }

    /// A convenience method returning the first place from a filtered version of this IL category's leaderboard.
    ///
    /// If the world record is tied, this method returns whichever run the API lists first.
    ///
    /// If no run has been verified for the given level, category, and filter, `Ok(None)` is returned.
    fn filtered_wr(self, filter: &Filter) -> Result<Option<Run>> {
        let (level, category) = self;
        let mut lb = level.client.get_query::<_, _, _, _, Leaderboard>(format!("/leaderboards/{}/level/{}/{}", level.game()?.id(), level.id(), category.id()), filter)?
            .runs;
        if lb.is_empty() { return Ok(None); }
        Ok(Some(level.client.annotate(lb.remove(0).run)))
    }

    /// Returns true if the world record for this level, category, and filter is tied.
    fn filtered_wr_is_tied(self, filter: &Filter) -> Result<bool> {
        let (level, category) = self;
        let lb = level.client.get_query::<_, _, _, _, Leaderboard>(format!("/leaderboards/{}/level/{}/{}", level.game()?.id(), level.id(), category.id()), filter)?
            .runs;
        Ok(lb.len() > 1 && lb[1].place == 1)
    }
}
