use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpConfig {
    #[serde(with = "humantime_serde")]
    pub connect_timeout: Duration,
    #[serde(with = "humantime_serde")]
    pub request_timeout: Duration,
    pub pool_max_idle_per_host: usize,
    pub proxy: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NlpConfig {
    pub enabled: bool,
    pub keywords: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub db_path: String,
    pub export_path: String,
    pub http: HttpConfig,
    pub nlp: NlpConfig,
}

impl Config {
    pub fn from_path(path: &str) -> Result<Self, anyhow::Error> {
        let file = std::fs::File::open(path)?;
        let config: Config = serde_yaml::from_reader(file)?;
        Ok(config)
    }

    pub fn get_concurrency(&self) -> (usize, usize) {
        // This is a placeholder for a more sophisticated concurrency model
        (32, 1)
    }
}