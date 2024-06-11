use clap::Parser;
use config::Config;
use fhir::Entry;
use fhir::Extension;
use fhir::Resource;
use fhir::Root;
use once_cell::sync::Lazy;
use reqwest::{Client, Error};
use rustls::pki_types;
use serde::Deserialize;
use serde::Serialize;
use serde_json::from_slice;

mod fhir;

mod config;

static CONFIG: Lazy<Config> = Lazy::new(Config::parse);

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct MaReadPatient {
    tokenType: String,
    data: MaReadPatientData,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct MaReadPatientData {
    searchIds: Vec<MaReadPatientDataIds>,
    resultIds: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct MaReadPatientDataIds {
    idString: String,
    idType: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct MaToken {
    id: String,
    url: String,
}

struct Project(String, String);

#[tokio::main]
async fn main() {
    println!("Starting Patient-Project-Indentificator...");

    //Use normal client in prod
    let client = reqwest::ClientBuilder::new()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();

    let projects = [
        Project("DKTK000000791".to_string(), "ReKo".to_string()),
        Project("DKTK000002089".to_string(), "EXLIQUID".to_string()),
        Project("DKTK000005001".to_string(), "CRC-Advanced".to_string()),
        Project("DKTK000004877".to_string(), "Meth4CRC".to_string()),
        Project("DKTK000002016".to_string(), "NeoLung".to_string()),
        Project("DKTK000001986".to_string(), "MASTER-Programm".to_string()),
        Project("DKTK000001985".to_string(), "RiskY-AML".to_string()),
        Project("DKTK000001951".to_string(), "ARMANI".to_string()),
        Project("DKTK000001950".to_string(), "IRCC".to_string()),
        Project("DKTK000001087".to_string(), "TamoBreastCa".to_string()),
        Project("DKTK000000899".to_string(),"Cov2Cancer-Register".to_string(),),
    ];

    let url = ma_session(&client).await;

    println!("{}", url);

    for project in projects {
        let token = ma_token_request(&client, url.clone(), &project.0).await;
        let patients = get_patient(&client, token).await;
        for patient in &patients {
            let mut fhirPatient = get_patient_from_blaze(&client, patient.to_string()).await;

            if let Some(ref mut extension) = fhirPatient.extension {
                if(!extension.contains(&Extension{url: "http://dktk.dkfz.de/fhir/Projects/".to_string() + &project.0.as_str()})) {
                    extension.push(Extension{url: "http://dktk.dkfz.de/fhir/Projects/".to_string() + &project.0.as_str()})
                }
            } else {
                fhirPatient.extension = Some(vec![Extension{url: "http://dktk.dkfz.de/fhir/Projects/".to_string() + &project.0.as_str()}]);
            }
            post_patient_to_blaze(&client, fhirPatient).await;
        }
    }
}

// 1. Get Mainzelliste Session
async fn ma_session(client: &Client) -> String {
    let res = client
        .post(
            CONFIG
                .mainzelliste_url
                .join("/patientlist/sessions")
                .unwrap(),
        )
        .header("mainzellisteApiKey", &CONFIG.mainzelliste_apikey)
        .send()
        .await
        .unwrap();

    res.error_for_status_ref().unwrap();

    let result = res
        .headers()
        .get("location")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    return result;
}

async fn ma_token_request(client: &Client, url: String, project: &String) -> String {
    let mrdataids = MaReadPatientDataIds {
        idString: "*".to_owned(),
        idType: format!("{}_{}_L-ID", project.to_string(), CONFIG.site_name),
    };

    let mrdata = MaReadPatientData {
        resultIds: vec![format!("BK_{}_L-ID", CONFIG.site_name)],
        searchIds: vec![mrdataids],
    };

    let body = MaReadPatient {
        tokenType: "readPatients".to_owned(),
        data: mrdata,
    };

    println!("{}", serde_json::to_string(&body).unwrap());

    let res = client.post(url).json(&body)
    .header("mainzellisteApiKey", &CONFIG.mainzelliste_apikey)
    .header("Content-Type", "application/json")
    .send().await.unwrap();

    res.error_for_status_ref().unwrap();

    dbg!(res);

    return "nix".to_string();
}

async fn get_patient(client: &Client, token: String) -> Vec<String> {
    // let res = reqwest::get(CONFIG.mainzelliste_url.join(&format!(" /patients/tokenId/{}", token)));
    // return res;

    return  vec!["ffb5d31a2e40afce286f9e1e70e29e19".to_string()];
}

async fn get_patient_from_blaze(client: &Client, patient_id: String) -> Resource {
    print!("Getting Patient");

    let res = client.get(
        CONFIG.blaze_url.join(
            &format!("/fhir/Patient?identifier={}", patient_id)).unwrap()
        
    ).send()
    .await
    .unwrap()
    ;


    return res.json::<Root>().await.unwrap().entry[0].resource.clone();
}

async fn post_patient_to_blaze(client: &Client, patient :Resource) {
    println!("Posting Patient");

    let res = client
    .put(
        CONFIG.blaze_url.join(
            &format!("/fhir/Patient/{}", patient.id)).unwrap()
    )
    .json(&patient)
    .send()
    .await
    .unwrap();

    dbg!(res);

}
