
//TODO: make custom fields customizable?

//in our jira, customfield_10000 is an array of these to represent Flagged..
/*
            "customfield_10000": [
                {
                    "disabled": false,
                    "id": "10000",
                    "self": "https://....",
                    "value": "Impediment",
                },
            ],

 */


use std::fmt::{Display, Formatter};
use std::collections::BTreeMap;
use serde_json::Value;

#[derive(Deserialize, Debug)]
pub struct Flagged {
    pub disabled: bool,
    pub id: String,
    #[serde(rename = "self")]
    pub self_link: String,
    pub value: String
}

fn parse_flags(value: serde_json::Value) -> bool {
    let flags: Vec<Flagged> = serde_json::from_value(value).unwrap_or_default();
    flags.iter().any(|f| f.value == "Impediment")
}

pub(crate) struct Flag(bool);
impl From<bool> for Flag {
    fn from(b: bool) -> Self {
        Flag(b)
    }
}
impl From<&BTreeMap<String, ::serde_json::Value>> for Flag {
    fn from(issuefields: &BTreeMap<String, Value>) -> Self {
        issuefields.get("customfield_10000").map_or(false, |f| parse_flags(f.clone())).into()
    }
}
impl From<&Flag> for bool {
    fn from(f: &Flag) -> Self {
        f.0
    }
}
impl Display for Flag {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.into() {
            write!(f, "{}", "🚩 ")
        } else {
            Ok(())
        }
    }
}