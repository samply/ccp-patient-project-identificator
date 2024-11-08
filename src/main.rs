use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use config::Config;
use fhir::Extension;
use fhir::Resource;
use fhir::Root;
use once_cell::sync::Lazy;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderName;
use reqwest::Client;
use serde_json::Value;
use tokio::time::sleep;

mod fhir;
mod mainzelliste;

mod config;

static CONFIG: Lazy<Config> = Lazy::new(Config::parse);

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
    println!("Starting Patient-Project-Indentificator...");

    //Use normal client in prod
    let mainzel_client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
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

    let fhir_client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true) // Mainzelliste returns full server url, some sites do not have a SSL Cert for their servers
        .build()?;

    wait_for_fhir_server(&fhir_client).await;

    let session_id = ma_session(&mainzel_client).await?;

    for project in projects {
        println!("Adding project information to patients of {}", project.name);
        for id_type in ["L", "G"] {
            let token =
                match ma_token_request(&mainzel_client, &session_id, &project, &id_type).await {
                    Ok(url) => url,
                    Err(_e) => {
                        eprintln!("Project {} {} not configured in mainzelliste", project.name, id_type);
                        continue;
                    }
                };
            let Ok(patients) = get_patient(&mainzel_client, token).await else {
                println!("Did not found any patients from project {} {id_type}", project.name);
                continue;
            };

            println!("Found {} patients from project {} {id_type}", patients.len(), project.name);

            for patient in &patients {
                let fhir_patient =
                    get_patient_from_fhir_server(&fhir_client, patient.to_string()).await;

                match fhir_patient {
                    Ok(mut fhir_patient) => {
                        let project_extension = Extension {
                            url: format!(
                                "http://dktk.dkfz.de/fhir/projects/{}",
                                project.id.as_str()
                            ),
                        };
                        if !fhir_patient.extension.contains(&project_extension) {
                            fhir_patient.extension.push(project_extension);
                        }
                        if let Err(e) =
                            post_patient_to_fhir_server(&fhir_client, fhir_patient).await
                        {
                            eprintln!("Failed to post patient: {e}\n{patient:#}");
                        } else {
                            println!("Added project to Patient {}", patient);
                        }
                    }
                    Err(e) => {
                        eprintln!("Did not find patient with pseudonym {}\n{:#}", &patient, e);
                    }
                }
            }
        }
    }
    tokio::time::sleep(Duration::MAX).await;
    Ok(())
}

// 1. Get Mainzelliste Session
async fn ma_session(client: &Client) -> anyhow::Result<String> {
    let res = client
        .post(CONFIG.mainzelliste_url.join("/patientlist/sessions")?)
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
        username: "test".to_owned(),
        remote_system: "test".to_owned(),
        reason_for_change: "keine".to_owned(),
    };

    let mrdata = mainzelliste::Data {
        result_ids: vec![format!("BK_{}_{}-ID", CONFIG.site_name, id_type)],
        search_ids: vec![mrdataids],
        audit_trail: audit,
    };

    let body = mainzelliste::Token {
        type_field: "readPatients".to_owned(),
        data: mrdata,
    };

    let res = client
        .post(
            CONFIG
                .mainzelliste_url
                .join(&format!("/patientlist/sessions/{}/tokens", session_id))?,
        )
        .json(&body)
        .send()
        .await?
        .error_for_status()?;

    res.json::<serde_json::Value>()
        .await?
        .get("tokenId")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or(anyhow::anyhow!("Got no token"))
}

async fn get_patient(client: &Client, token: String) -> anyhow::Result<Vec<String>> {
    Ok(client
        .get(
            CONFIG
                .mainzelliste_url
                .join(&format!("/patientlist/patients/tokenId/{token}"))?,
        )
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<Value>>()
        .await?
        .into_iter()
        .filter_map(|v| v["ids"][0]["idString"].as_str().map(ToOwned::to_owned))
        .collect())
}

async fn get_patient_from_fhir_server(
    client: &Client,
    patient_id: String,
) -> anyhow::Result<Resource> {
    let res = client
        .get(
            CONFIG
                .fhir_server_url
                .join(&format!("/fhir/Patient?identifier={}", patient_id))
                .unwrap(),
        )
        .send()
        .await
        .context("Could not reach fhir_server")?
        .error_for_status()
        .context("Unsuccessful status code")?;

    Ok(res
        .json::<Root>()
        .await
        .context("Fail to parse patient resource")?
        .entry
        .first()
        .ok_or_else(|| anyhow::anyhow!("Could not find any patient"))?
        .resource
        .clone())
}

async fn post_patient_to_fhir_server(client: &Client, patient: Resource) -> anyhow::Result<()> {
    client
        .put(
            CONFIG
                .fhir_server_url
                .join(&format!("/fhir/Patient/{}", patient.id))
                .unwrap(),
        )
        .json(&patient)
        .send()
        .await
        .context("Could not reach fhir_server")?
        .error_for_status()?;
    Ok(())
}

async fn wait_for_fhir_server(client: &Client) {
    loop {
        if client
            .get(CONFIG.fhir_server_url.join("/fhir/metadata").unwrap())
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
        {
            break;
        }
        println!("Waiting for fhir server startup");
        sleep(Duration::from_secs(10)).await;
    }
}
