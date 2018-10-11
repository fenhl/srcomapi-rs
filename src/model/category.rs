//! Categories are the different rulesets for speedruns

use std::fmt;
use super::super::{
    Result,
    client::{
        AnnotatedData,
        Client,
        ResponseData
    }
};

/// The cached data for a category. This type is an implementation detail. You're probably looking for `Category` instead.
#[derive(Debug, Deserialize)]
pub struct CategoryData {
    id: String,
    name: String
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

    /// Returns this category's API ID.
    pub fn id(&self) -> &str {
        &self.data.id
    }
}


/// Displays the category name.
impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.data.name.fmt(f)
    }
}
