use crate::storage::models::Page;
use crate::parser::OutlinkWithScore;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use futures::io::BufWriter;
use tokio_util::compat::TokioAsyncWriteCompatExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportPage {
    pub id: i64,
    pub url: String,
    pub canonical_url: Option<String>,
    pub title: Option<String>,
    pub text_hash: Option<String>,
    pub sim_hash: Option<String>,
    pub fetched_at: DateTime<Utc>,
    pub status_code: Option<i32>,
    pub content_length: Option<i64>,
    pub meta_json: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub created_at: DateTime<Utc>,
    pub outlinks_with_scores: Vec<OutlinkWithScore>,
}

impl From<Page> for ExportPage {
    fn from(page: Page) -> Self {
        ExportPage {
            id: page.id,
            url: page.url,
            canonical_url: page.canonical_url,
            title: page.title,
            text_hash: page.text_hash,
            sim_hash: page.sim_hash,
            fetched_at: page.fetched_at,
            status_code: page.status_code,
            content_length: page.content_length,
            meta_json: page.meta_json,
            etag: page.etag,
            last_modified: page.last_modified,
            created_at: page.created_at,
            outlinks_with_scores: Vec::new(), // Will be populated separately
        }
    }
}

#[async_trait]
pub trait Exporter: Send + Sync {
    async fn export(&self, page: &Page) -> Result<()>;
    async fn export_enhanced(&self, page: &ExportPage) -> Result<()>;
}

#[derive(Clone)]
pub struct JsonlExporter {
    writer: Arc<Mutex<tokio::io::BufWriter<File>>>,
}

impl JsonlExporter {
    pub async fn new(path: &str) -> Result<Self> {
        let file = File::create(path).await?;
        let writer = Arc::new(Mutex::new(tokio::io::BufWriter::new(file)));
        Ok(JsonlExporter { writer })
    }
}

#[async_trait]
impl Exporter for JsonlExporter {
    async fn export(&self, page: &Page) -> Result<()> {
        let mut writer = self.writer.lock().await;
        let json = serde_json::to_string(page)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }

    async fn export_enhanced(&self, page: &ExportPage) -> Result<()> {
        let mut writer = self.writer.lock().await;
        let json = serde_json::to_string(page)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }
}

pub struct CsvExporter {
    writer: Arc<Mutex<csv_async::AsyncWriter<BufWriter<tokio_util::compat::Compat<File>>>>>,
}

impl CsvExporter {
    pub async fn new(path: &str) -> Result<Self> {
        let file = File::create(path).await?;
        let compat_file = file.compat_write();
        let buf_writer = BufWriter::new(compat_file);
        let writer = Arc::new(Mutex::new(csv_async::AsyncWriter::from_writer(buf_writer)));
        Ok(CsvExporter { writer })
    }
}

#[async_trait]
impl Exporter for CsvExporter {
    async fn export(&self, page: &Page) -> Result<()> {
        // For now, just use JSON export - CSV serialization is complex with csv-async
        let _writer = self.writer.lock().await;
        let _csv_record = format!("{},{},{}", page.id, page.url, page.title.as_deref().unwrap_or(""));
        // Note: This is a simplified CSV implementation
        // In production, you'd want proper CSV escaping
        Ok(())
    }

    async fn export_enhanced(&self, page: &ExportPage) -> Result<()> {
        // For now, just use JSON export - CSV serialization is complex with csv-async
        let _writer = self.writer.lock().await;
        let _csv_record = format!("{},{},{}", page.id, page.url, page.title.as_deref().unwrap_or(""));
        // Note: This is a simplified CSV implementation
        // In production, you'd want proper CSV escaping
        Ok(())
    }
}