use std::sync::LazyLock;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use config::Config;
use fhir::Bundle;
use fhir::Patient;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderName;
use reqwest::Client;
use reqwest::Url;
use serde_json::Value;
use tokio::time::sleep;
use tracing::error;
use tracing::info;
use tracing::warn;
use tracing_subscriber::EnvFilter;

mod fhir;
mod mainzelliste;

mod config;

static CONFIG: LazyLock<Config> = LazyLock::new(Config::parse);

const RUN_INTERVAL: Duration = Duration::from_secs(60 * 60 * 24);

struct Project {
    id: String,
    name: String,
}

impl Project {
    fn new(id: String, name: String) -> Self {
        Self { id, name }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    info!("Starting Patient-Project-Identificator");

    let mainzel_client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(!CONFIG.require_signed_certs)
        .default_headers(HeaderMap::from_iter([(
            HeaderName::from_static("mainzellisteapikey"),
            CONFIG.mainzelliste_apikey.clone(),
        )]))
        .build()?;

    let projects = [
        Project::new("DKTK000000791".to_string(), "ReKo".to_string()),
        Project::new("DKTK000002089".to_string(), "EXLIQUID".to_string()),
        Project::new("DKTK000005001".to_string(), "CRC-Advanced".to_string()),
        Project::new("DKTK000004877".to_string(), "Meth4CRC".to_string()),
        Project::new("DKTK000002016".to_string(), "NeoLung".to_string()),
        Project::new("DKTK000001986".to_string(), "MASTER-Programm".to_string()),
        Project::new("DKTK000001985".to_string(), "RiskY-AML".to_string()),
        Project::new("DKTK000001951".to_string(), "ARMANI".to_string()),
        Project::new("DKTK000001950".to_string(), "IRCC".to_string()),
        Project::new("DKTK999999999".to_string(), "Testprojekt".to_string()),
        Project::new("DKTK000001087".to_string(), "TamoBreastCa".to_string()),
        Project::new(
            "DKTK000000899".to_string(),
            "Cov2Cancer-Register".to_string(),
        ),
    ];

    // Mainzelliste returns the full server url, some sites do not have a signed
    // certificate for their servers.
    let fhir_client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(!CONFIG.require_signed_certs)
        .build()?;

    loop {
        wait_for_fhir_server(&fhir_client).await;

        let session_id = ma_session(&mainzel_client).await?;

        for project in &projects {
            info!("Adding project information to patients of {}", project.name);
            for id_type in ["L", "G"] {
                let token =
                    match ma_token_request(&mainzel_client, &session_id, project, id_type).await {
                        Ok(token) => token,
                        Err(e) => {
                            error!(
                                "Could not get a readPatients token for project {} {id_type}, \
                                 it is probably not configured in the Mainzelliste: {e:#}",
                                project.name
                            );
                            continue;
                        }
                    };

                let patients = match get_patients(&mainzel_client, &token, id_type).await {
                    Ok(patients) => patients,
                    Err(e) => {
                        error!(
                            "Could not read the patients of project {} {id_type}: {e:#}",
                            project.name
                        );
                        continue;
                    }
                };

                info!(
                    "Found {} patients from project {} {id_type}",
                    patients.len(),
                    project.name
                );

                let extension_url =
                    format!("http://dktk.dkfz.de/fhir/projects/{}", project.id.as_str());

                for pseudonym in &patients {
                    let mut fhir_patient =
                        match get_patient_from_fhir_server(&fhir_client, pseudonym).await {
                            Ok(patient) => patient,
                            Err(e) => {
                                error!("Did not find patient with pseudonym {pseudonym}: {e:#}");
                                continue;
                            }
                        };

                    if fhir_patient.has_extension(&extension_url) {
                        continue;
                    }

                    if let Err(e) = fhir_patient.add_extension(&extension_url) {
                        error!("Could not tag patient {pseudonym}: {e:#}");
                        continue;
                    }

                    match post_patient_to_fhir_server(&fhir_client, &fhir_patient).await {
                        Ok(()) => info!("Added project to patient {pseudonym}"),
                        Err(e) => error!("Failed to write patient {pseudonym}: {e:#}"),
                    }
                }
            }
        }
        sleep(RUN_INTERVAL).await;
    }
}

/// Appends a relative path to a configured base url.
///
/// [`Url::join`] replaces the whole path when the argument starts with a slash
/// and drops the last segment when the base has no trailing slash, so a server
/// hosted under a sub path would silently be addressed wrong.
fn join_url(base: &Url, path: &str) -> anyhow::Result<Url> {
    let mut url = base.clone();
    url.path_segments_mut()
        .map_err(|()| anyhow::anyhow!("The url {base} cannot be used as a base"))?
        .pop_if_empty()
        .push("");
    url.join(path.trim_start_matches('/'))
        .with_context(|| format!("Could not append {path} to {base}"))
}

// 1. Get Mainzelliste Session
async fn ma_session(client: &Client) -> anyhow::Result<String> {
    let res = client
        .post(join_url(&CONFIG.mainzelliste_url, "patientlist/sessions")?)
        .send()
        .await?
        .error_for_status()?;

    Ok(res
        .headers()
        .get("location")
        .ok_or(anyhow::anyhow!("No location header"))?
        .to_str()?
        .trim_end_matches('/')
        .rsplit_once('/')
        .ok_or(anyhow::anyhow!("No Session ID"))?
        .1
        .to_string())
}

async fn ma_token_request(
    client: &Client,
    session_id: &str,
    project: &Project,
    id_type: &str,
) -> anyhow::Result<String> {
    let mrdataids = mainzelliste::SearchId {
        id_string: "*".to_owned(),
        id_type: format!("{}_{}_{}-ID", project.id, CONFIG.site_name, id_type),
    };

    let audit = mainzelliste::AuditTrail {
        username: "project user".to_owned(),
        remote_system: "ccp-ppi".to_owned(),
        reason_for_change: "no changes made".to_owned(),
    };

    let mrdata = mainzelliste::Data {
        result_ids: vec![result_id_type(id_type)],
        search_ids: vec![mrdataids],
        audit_trail: audit,
    };

    let body = mainzelliste::Token {
        type_field: "readPatients".to_owned(),
        data: mrdata,
    };

    let res = client
        .post(join_url(
            &CONFIG.mainzelliste_url,
            &format!("patientlist/sessions/{session_id}/tokens"),
        )?)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    res.json::<Value>()
        .await?
        .get("tokenId")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or(anyhow::anyhow!("Got no token"))
}

/// The id type this component asks the Mainzelliste to return, which is the
/// pseudonym the FHIR server stores as the patient identifier.
fn result_id_type(id_type: &str) -> String {
    format!("BK_{}_{}-ID", CONFIG.site_name, id_type)
}

async fn get_patients(client: &Client, token: &str, id_type: &str) -> anyhow::Result<Vec<String>> {
    let wanted = result_id_type(id_type);

    Ok(client
        .get(join_url(
            &CONFIG.mainzelliste_url,
            &format!("patientlist/patients/tokenId/{token}"),
        )?)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<mainzelliste::Patient>>()
        .await?
        .into_iter()
        .filter_map(|patient| {
            patient
                .ids
                .into_iter()
                .find(|id| id.id_type == wanted)
                .map(|id| id.id_string)
        })
        .collect())
}

async fn get_patient_from_fhir_server(
    client: &Client,
    patient_id: &str,
) -> anyhow::Result<Patient> {
    let mut url = join_url(&CONFIG.fhir_server_url, "fhir/Patient")?;
    url.query_pairs_mut().append_pair("identifier", patient_id);

    let res = client
        .get(url)
        .send()
        .await
        .context("Could not reach fhir_server")?
        .error_for_status()
        .context("Unsuccessful status code")?;

    let entries = res
        .json::<Bundle>()
        .await
        .context("Failed to parse patient resource")?
        .entry;

    if entries.len() > 1 {
        warn!(
            "Pseudonym {patient_id} matches {} patients in the FHIR server, only the first one \
             will be tagged",
            entries.len()
        );
    }

    Ok(entries
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not find any patient"))?
        .resource)
}

async fn post_patient_to_fhir_server(client: &Client, patient: &Patient) -> anyhow::Result<()> {
    let url = join_url(
        &CONFIG.fhir_server_url,
        &format!("fhir/Patient/{}", patient.id()?),
    )?;

    client
        .put(url)
        .json(patient)
        .send()
        .await
        .context("Could not reach fhir_server")?
        .error_for_status()?;
    Ok(())
}

async fn wait_for_fhir_server(client: &Client) {
    let url = match join_url(&CONFIG.fhir_server_url, "fhir/metadata") {
        Ok(url) => url,
        Err(e) => {
            error!("Invalid FHIR server url: {e:#}");
            return;
        }
    };

    loop {
        if client
            .get(url.clone())
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            break;
        }
        info!("Waiting for fhir server startup");
        sleep(Duration::from_secs(10)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn join_url_keeps_the_base_path() {
        let base = Url::parse("https://example.com/bridgehead").unwrap();
        assert_eq!(
            join_url(&base, "fhir/metadata").unwrap().as_str(),
            "https://example.com/bridgehead/fhir/metadata"
        );
        // a trailing slash on the base must not double up
        let base = Url::parse("https://example.com/bridgehead/").unwrap();
        assert_eq!(
            join_url(&base, "fhir/metadata").unwrap().as_str(),
            "https://example.com/bridgehead/fhir/metadata"
        );
    }

    #[test]
    fn tagging_a_patient_keeps_unknown_fields() {
        let mut patient: Patient = serde_json::from_value(json!({
            "resourceType": "Patient",
            "id": "abc",
            "address": [{ "city": "Heidelberg" }],
            "extension": [{ "url": "http://dktk.dkfz.de/fhir/projects/DKTK000000791" }],
        }))
        .unwrap();

        assert!(patient.has_extension("http://dktk.dkfz.de/fhir/projects/DKTK000000791"));
        assert!(!patient.has_extension("http://dktk.dkfz.de/fhir/projects/DKTK000002089"));

        patient
            .add_extension("http://dktk.dkfz.de/fhir/projects/DKTK000002089")
            .unwrap();

        assert_eq!(
            serde_json::to_value(&patient).unwrap(),
            json!({
                "resourceType": "Patient",
                "id": "abc",
                "address": [{ "city": "Heidelberg" }],
                "extension": [
                    { "url": "http://dktk.dkfz.de/fhir/projects/DKTK000000791" },
                    { "url": "http://dktk.dkfz.de/fhir/projects/DKTK000002089" },
                ],
            })
        );
    }

    #[test]
    fn a_patient_without_extensions_gets_the_array() {
        let mut patient: Patient =
            serde_json::from_value(json!({ "resourceType": "Patient", "id": "abc" })).unwrap();

        assert!(!patient.has_extension("http://dktk.dkfz.de/fhir/projects/DKTK000000791"));
        patient
            .add_extension("http://dktk.dkfz.de/fhir/projects/DKTK000000791")
            .unwrap();
        assert!(patient.has_extension("http://dktk.dkfz.de/fhir/projects/DKTK000000791"));
    }
}
