//! Variables are custom criteria to distinguish between runs done in the same category or level

use std::{
    collections::{
        BTreeMap,
        HashMap
    },
    fmt,
    hash::Hash,
    iter::FromIterator
};
use serde_derive::{
    Deserialize,
    Serialize
};
use crate::{
    Result,
    client::{
        AnnotatedData,
        Client,
        ResponseData
    }
};

#[derive(Debug, Deserialize, Clone)]
struct ValueData {
    label: String,
    rules: Option<String>,
    //#[serde(default)]
    //flags: HashMap<String, bool> //TODO apparently this sometimes has nulls in it? Need to figure out how to handle those
}

/// A possible value of a variable.
#[derive(Debug)]
pub struct Value {
    id: String,
    inner: ValueData
}

impl Value {
    /// Returns the value's API ID.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the label, or human-readable name, of the value.
    pub fn label(&self) -> &str {
        &self.inner.label
    }

    /// If this is a subcategory, returns the subcategory's rules.
    pub fn rules(&self) -> Option<&str> {
        self.inner.rules.as_ref().map(|rules_buf| &rules_buf[..])
    }

    /*
    /// If this is a subcategory, returns whether or not it is considered miscellaneous, i.e. hidden behind a “more” button by default.
    pub fn is_misc(&self) -> Option<bool> {
        self.inner.flags.get("miscellaneous").cloned()
    }
    */
}

#[derive(Debug, Deserialize, Clone)]
struct ValuesData {
    values: HashMap<String, ValueData>,
    default: Option<String>
}

/// The cached data for a variable. This type is an implementation detail. You're probably looking for `Variable` instead.
#[derive(Debug, Deserialize, Clone)]
pub struct VariableData {
    id: String,
    name: String,
    values: ValuesData
}

/// Variables are custom criteria to distinguish between runs done in the same category or level.
pub type Variable = AnnotatedData<VariableData>;

impl Variable {
    /// Returns the variable with the given ID.
    pub fn from_id(client: &Client, id: impl fmt::Display) -> Result<Variable> {
        Ok(client.annotate(
            client.get(format!("/variables/{}", id))?
                .send()?
                .error_for_status()?
                .json::<ResponseData<_>>()?
                .data
        ))
    }

    /// Returns this variable's API ID.
    pub fn id(&self) -> &str {
        &self.data.id
    }

    /// Returns the list of possible values this variable can be.
    pub fn values(&self) -> Vec<Value> {
        self.data.values.values.iter()
            .map(|(value_id, value_data)| Value {
                id: value_id.to_owned(),
                inner: value_data.clone()
            })
            .collect()
    }

    /// Returns the default value of this variable, if defined.
    pub fn default_value(&self) -> Option<Value> {
        self.data.values.default.as_ref().map(|default_id| Value {
            id: default_id.to_owned(),
            inner: self.data.values.values[default_id].clone()
        })
    }
}

/// Displays the variable name.
impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.data.name.fmt(f)
    }
}

/// This type is used to filter leaderboards by variable/value pairs via the `Category::leaderboard_filtered` method.
#[derive(Debug, Default, Clone, Serialize)]
pub struct Filter(HashMap<String, String>);

impl<K: fmt::Display, V: ToString> From<BTreeMap<K, V>> for Filter {
    fn from(map: BTreeMap<K, V>) -> Filter {
        Filter(map.into_iter().map(|(var_id, value_id)| (format!("var-{}", var_id), value_id.to_string())).collect())
    }
}

impl<K: Eq + Hash + fmt::Display, V: ToString> From<HashMap<K, V>> for Filter {
    fn from(map: HashMap<K, V>) -> Filter {
        Filter(map.into_iter().map(|(var_id, value_id)| (format!("var-{}", var_id), value_id.to_string())).collect())
    }
}

impl<K: fmt::Display, V: ToString> FromIterator<(K, V)> for Filter {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(i: I) -> Filter {
        Filter(i.into_iter().map(|(var_id, value_id)| (format!("var-{}", var_id), value_id.to_string())).collect())
    }
}
