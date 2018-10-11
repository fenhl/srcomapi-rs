use reqwest::Url;

#[derive(Deserialize)]
#[serde(remote = "Url")]
pub(crate) struct UrlDef(#[serde(getter = "Url::into_string")] String);

impl From<UrlDef> for Url {
    fn from(UrlDef(url_string): UrlDef) -> Url {
        Url::parse(&url_string).expect("invalid URL")
    }
}
