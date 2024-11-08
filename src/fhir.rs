use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(default)]
    pub entry: Vec<Entry>,
    pub link: Vec<Link>,
    pub total: i64,
    pub resource_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub full_url: String,
    pub resource: Resource,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Resource {
    pub meta: Meta,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birth_date: Option<String>,
    pub resource_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deceased_date_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deceased: Option<bool>,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<Vec<Identifier>>,
    #[serde(default)]
    pub extension: Option<Vec<Extension>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub version_id: String,
    pub last_updated: String,
    pub profile: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Identifier {
    #[serde(rename = "type")]
    pub type_field: Type,
    pub value: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Type {
    pub coding: Vec<Coding>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Coding {
    pub system: String,
    pub code: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Search {
    pub mode: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub relation: String,
    pub url: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Extension {
    pub url: String,
}
