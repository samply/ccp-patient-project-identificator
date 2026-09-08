use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Token {
    #[serde(rename = "type")]
    pub type_field: String,
    pub data: Data,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub search_ids: Vec<SearchId>,
    pub result_ids: Vec<String>,
    pub audit_trail: AuditTrail,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchId {
    pub id_string: String,
    pub id_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditTrail {
    pub username: String,
    pub remote_system: String,
    pub reason_for_change: String,
}

/// One entry of the patient list returned for a `readPatients` token.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Patient {
    #[serde(default)]
    pub ids: Vec<Id>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Id {
    pub id_type: String,
    pub id_string: String,
}
