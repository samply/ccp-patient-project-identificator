use clap::Parser;
use reqwest::Url;


#[derive(Parser, Clone, Debug)]
#[clap(author, version, about, long_about = None)]

pub struct Config {
    #[clap(long, env)]
    pub mainzelliste_url: Url,

    #[clap(long, env)]
    pub mainzelliste_apikey: String,

    #[clap(long, env)]
    pub site_name: String,

    #[clap(long, env)]
    pub blaze_url: Url,
}
