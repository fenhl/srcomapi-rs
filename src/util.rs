use std::time::Duration;
use bigdecimal::{
    BigDecimal,
    ToPrimitive,
    Zero
};
use lazy_static::lazy_static;
use regex::Regex;
use reqwest::Url;
use serde_derive::Deserialize;

lazy_static! {
    static ref DURATION_RE: Regex = Regex::new("^PT(?:([0-9.]+)H)?(?:([0-9.]+)M)?(?:([0-9.]+)S)?$").unwrap();
}

#[derive(Deserialize)]
#[serde(remote = "Duration")]
pub(crate) struct DurationDef(#[serde(getter = "unimplemented")] String);

impl From<DurationDef> for Duration {
    fn from(DurationDef(duration_string): DurationDef) -> Duration {
        let captures = DURATION_RE.captures(&duration_string).expect("invalid ISO 8601 duration");
        let hours = captures.get(1).map(|hours_match| hours_match.as_str().parse::<BigDecimal>().unwrap()).unwrap_or_else(BigDecimal::zero);
        let minutes = captures.get(2).map(|mins_match| mins_match.as_str().parse::<BigDecimal>().unwrap()).unwrap_or_else(BigDecimal::zero);
        let seconds = captures.get(3).map(|secs_match| secs_match.as_str().parse::<BigDecimal>().unwrap()).unwrap_or_else(BigDecimal::zero);
        let total_secs = (hours * BigDecimal::from(60) + minutes) * BigDecimal::from(60) + seconds;
        let nanos = (&total_secs % BigDecimal::from(1)) * BigDecimal::from(1_000_000_000);
        Duration::new(total_secs.to_u64().expect("duration too long"), nanos.to_u32().unwrap())
    }
}

type OptDuration = Option<Duration>;

#[derive(Deserialize)]
#[serde(remote = "OptDuration")]
pub(crate) struct OptDurationDef(#[serde(getter = "unimplemented")] Option<String>);

impl From<OptDurationDef> for Option<Duration> {
    fn from(OptDurationDef(opt_duration): OptDurationDef) -> Option<Duration> {
        opt_duration.map(|duration_string| DurationDef(duration_string).into())
    }
}

#[derive(Deserialize)]
#[serde(remote = "Url")]
pub(crate) struct UrlDef(#[serde(getter = "Url::into_string")] String);

impl From<UrlDef> for Url {
    fn from(UrlDef(url_string): UrlDef) -> Url {
        Url::parse(&url_string).expect("invalid URL")
    }
}
