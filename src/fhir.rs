use anyhow::Context;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use serde_json::Value;

/// A FHIR search response. Only the entries and the paging links are read, the
/// rest of the bundle is ignored.
#[derive(Debug, Clone, Deserialize)]
pub struct Bundle<T> {
    // `default = "Vec::new"` instead of `default`, which would require T: Default
    #[serde(default = "Vec::new")]
    pub entry: Vec<Entry<T>>,
    #[serde(default)]
    pub link: Vec<Link>,
}

impl<T> Bundle<T> {
    /// The url of the next page of a paged search result.
    pub fn next_link(&self) -> Option<&str> {
        self.link
            .iter()
            .find(|link| link.relation == "next")
            .map(|link| link.url.as_str())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Entry<T> {
    pub resource: T,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Link {
    pub relation: String,
    pub url: String,
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

/// A Specimen resource. Only the subject is read.
#[derive(Debug, Clone, Deserialize)]
#[serde(transparent)]
pub struct Specimen(Value);

impl Specimen {
    /// The logical id of the patient this specimen was taken from.
    pub fn patient_id(&self) -> anyhow::Result<&str> {
        let specimen_id = self.0["id"].as_str().unwrap_or("<no id>");
        let reference = self.0["subject"]["reference"]
            .as_str()
            .with_context(|| format!("Specimen {specimen_id} has no subject reference"))?;

        match reference.split('/').collect::<Vec<_>>().as_slice() {
            ["Patient", id] | ["Patient", id, "_history", _] => Ok(id),
            _ => anyhow::bail!(
                "Subject {reference} of specimen {specimen_id} is not a Patient reference"
            ),
        }
    }
}
