use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrawlifyError {
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("URL parsing error: {0}")]
    Url(#[from] url::ParseError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("XML parsing error: {0}")]
    Xml(#[from] quick_xml::DeError),
    #[error("Sitemap not found for domain: {0}")]
    SitemapNotFound(String),
    #[error("Tokio task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
    #[error("BB8 Rusqlite error: {0}")]
    BB8Rusqlite(#[from] bb8_rusqlite::Error),
    #[error("BB8 Pool error: {0}")]
    BB8Pool(#[from] bb8::RunError<bb8_rusqlite::Error>),
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("An unknown error has occurred")]
    Unknown,
}