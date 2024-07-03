use clap::Parser;
use config::Config;
use fhir::Resource;
use fhir::Root;
use once_cell::sync::Lazy;
use reqwest::header::HeaderMap;
use reqwest::header::HeaderName;
use reqwest::Client;
use serde_json::Value;

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
        .danger_accept_invalid_certs(true) // TODO: Remove
        .default_headers(HeaderMap::from_iter([(HeaderName::from_static("mainzellisteapikey"), CONFIG.mainzelliste_apikey.clone())]))
        .build()?;

    let projects = [
        Project::new("DKTK000000791".to_string(), "ReKo".to_string()),
        Project::new("DKTK000002089".to_string(), "EXLIQUID".to_string()),
        // Project::new("DKTK000005001".to_string(), "CRC-Advanced".to_string()),
        // Project::new("DKTK000004877".to_string(), "Meth4CRC".to_string()),
        // Project::new("DKTK000002016".to_string(), "NeoLung".to_string()),
        Project::new("DKTK000001986".to_string(), "MASTER-Programm".to_string()),
        Project::new("DKTK000001985".to_string(), "RiskY-AML".to_string()),
        Project::new("DKTK000001951".to_string(), "ARMANI".to_string()),
        Project::new("DKTK000001950".to_string(), "IRCC".to_string()),
        // Project::new("DKTK000001087".to_string(), "TamoBreastCa".to_string()),
        // Project::new("DKTK000000899".to_string(), "Cov2Cancer-Register".to_string()),
    ];

    let url = ma_session(&mainzel_client).await?;

    println!("{}", url);
    println!("-----------------------------");

    for project in projects {
        let token = match ma_token_request(&mainzel_client, &url, &project).await {
            Ok(url) => url,
            Err(e) => {
                eprintln!("Failed to get token url for {}: {e}", project.name);
                continue;
            },
        };
        let Ok(patients) = get_patient(&mainzel_client, token).await else { continue; };
        dbg!(patients);

        // for patient in &patients {
        //     let mut fhirPatient = get_patient_from_blaze(&client, patient.to_string()).await;

        //     if let Some(ref mut extension) = fhirPatient.extension {
        //         if(!extension.contains(&Extension{url: "http://dktk.dkfz.de/fhir/Projects/".to_string() + &project.0.as_str()})) {
        //             extension.push(Extension{url: "http://dktk.dkfz.de/fhir/Projects/".to_string() + &project.0.as_str()})
        //         }
        //     } else {
        //         fhirPatient.extension = Some(vec![Extension{url: "http://dktk.dkfz.de/fhir/Projects/".to_string() + &project.0.as_str()}]);
        //     }
        //     post_patient_to_blaze(&client, fhirPatient).await;
        // }
    }
    Ok(())
}

// 1. Get Mainzelliste Session
async fn ma_session(client: &Client) -> anyhow::Result<String> {
    let res = client
        .post(
            CONFIG
                .mainzelliste_url
                .join("/patientlist/sessions")?,
        )
        .send()
        .await?
        .error_for_status()?;

    Ok(res
        .headers()
        .get("location")
        .ok_or(anyhow::anyhow!("No location header"))?
        .to_str()?
        .to_string())
}

async fn ma_token_request(
    client: &Client,
    url: &str,
    project: &Project,
) -> anyhow::Result<String> {
    let mrdataids = mainzelliste::SearchId {
        id_string: "*".to_owned(),
        id_type: format!("{}_{}_L-ID", project.id, CONFIG.site_name),
    };

    let audit = mainzelliste::AuditTrail {
        username: "test".to_owned(),
        remote_system: "test".to_owned(),
        reason_for_change: "keine".to_owned(),
    };

    let mrdata = mainzelliste::Data {
        result_ids: vec![format!("BK_{}_L-ID", CONFIG.site_name)],
        search_ids: vec![mrdataids],
        audit_trail: audit,
    };

    let body = mainzelliste::Token {
        type_field: "readPatients".to_owned(),
        data: mrdata,
    };

    // println!("{}", serde_json::to_string(&body).unwrap());

    let res = client
        .post(format!("{url}tokens"))
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
        .get(CONFIG.mainzelliste_url.join(&format!("/patientlist/patients/tokenId/{token}"))?)
        .send()
        .await?
        .error_for_status()?
        .json::<Vec<Value>>()
        .await?
        .into_iter()
        .filter_map(|v| v["ids"][0]["idString"].as_str().map(ToOwned::to_owned))
        .collect()
    )
}

async fn get_patient_from_blaze(client: &Client, patient_id: String) -> Resource {
    print!("Getting Patient");

    let res = client
        .get(
            CONFIG
                .blaze_url
                .join(&format!("/fhir/Patient?identifier={}", patient_id))
                .unwrap(),
        )
        .send()
        .await
        .unwrap();

    return res.json::<Root>().await.unwrap().entry[0].resource.clone();
}

async fn post_patient_to_blaze(client: &Client, patient: Resource) {
    println!("Posting Patient");

    let res = client
        .put(
            CONFIG
                .blaze_url
                .join(&format!("/fhir/Patient/{}", patient.id))
                .unwrap(),
        )
        .json(&patient)
        .send()
        .await
        .unwrap();

    dbg!(res);
}
