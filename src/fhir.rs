use anyhow::Context;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;

/// A FHIR search response. Only the entries are read, the rest of the bundle is
/// ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct Bundle {
    #[serde(default)]
    pub entry: Vec<Entry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Entry {
    pub resource: Patient,
}

/// A Patient resource kept as raw JSON. Writing a patient back is a full
/// replace, so keeping the untouched JSON makes sure no field is dropped just
/// because this component does not model it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Patient(Value);

impl Patient {
    pub fn id(&self) -> anyhow::Result<&str> {
        self.0["id"].as_str().context("Patient resource has no id")
    }

    pub fn has_extension(&self, url: &str) -> bool {
        self.0["extension"]
            .as_array()
            .is_some_and(|extensions| extensions.iter().any(|e| e["url"].as_str() == Some(url)))
    }

    /// Appends a bare url extension. The CQL that consumes this only matches on
    /// `Patient.extension.url`, so no value element is set.
    pub fn add_extension(&mut self, url: &str) -> anyhow::Result<()> {
        self.0
            .as_object_mut()
            .context("Patient resource is not a JSON object")?
            .entry("extension")
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .context("Patient.extension is not an array")?
            .push(json!({ "url": url }));
        Ok(())
    }
}
