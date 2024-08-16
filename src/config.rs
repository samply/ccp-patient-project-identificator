use clap::Parser;
use reqwest::{header::HeaderValue, Url};

#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]

pub struct Config {
    #[clap(long, env, default_value = "http://bridgehead-patientlist:8080")]
    pub mainzelliste_url: Url,

    #[clap(long, env)]
    pub mainzelliste_apikey: HeaderValue,

    #[clap(long, env)]
    pub site_name: String,

    #[clap(long, env, default_value = "http://bridgehead-ccp-blaze:8080")]
    pub fhir_server_url: Url,
}
