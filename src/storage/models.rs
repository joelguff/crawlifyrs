use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as RusqliteResult, Row};
use crate::error::CrawlifyError;

type Result<T> = std::result::Result<T, CrawlifyError>;

#[derive(Debug)]
pub enum StorageError {
    NotFound,
    QueryError(String),
}

impl From<StorageError> for CrawlifyError {
    fn from(err: StorageError) -> Self {
        CrawlifyError::Database(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ABORT),
            Some(format!("{:?}", err))
        ))
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum CrawlMethod {
    DEFAULT,
    NLP,
    HEADERS,
    CHANGED,
}

impl CrawlMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            CrawlMethod::DEFAULT => "DEFAULT",
            CrawlMethod::NLP => "NLP",
            CrawlMethod::HEADERS => "HEADERS",
            CrawlMethod::CHANGED => "CHANGED",
        }
    }
}

impl From<&str> for CrawlMethod {
    fn from(s: &str) -> Self {
        match s {
            "NLP" => CrawlMethod::NLP,
            "HEADERS" => CrawlMethod::HEADERS,
            "CHANGED" => CrawlMethod::CHANGED,
            _ => CrawlMethod::DEFAULT,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Scope {
    pub id: i64,
    pub pattern: String,
    pub method: CrawlMethod,
    pub keywords: Option<String>,
    pub is_active: bool,
    pub last_crawled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl Scope {
    pub fn from_row(row: &Row) -> RusqliteResult<Self> {
        let method_str: String = row.get("method")?;
        Ok(Scope {
            id: row.get("id")?,
            pattern: row.get("pattern")?,
            method: CrawlMethod::from(method_str.as_str()),
            keywords: row.get("keywords")?,
            is_active: row.get("is_active")?,
            last_crawled_at: row.get("last_crawled_at")?,
            created_at: row.get("created_at")?,
        })
    }

    pub fn create(conn: &Connection, pattern: &str) -> Result<Self> {
        conn.execute(
            "INSERT INTO scopes (pattern) VALUES (?1)",
            params![pattern],
        )?;
        let id = conn.last_insert_rowid();
        Self::find_by_id(conn, id)
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Self> {
        Ok(conn.query_row("SELECT * FROM scopes WHERE id = ?1", params![id], Self::from_row)
            .map_err(|_| StorageError::NotFound)?)
    }

    pub fn get_active(conn: &Connection) -> Result<Vec<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM scopes WHERE is_active = 1")?;
        let scopes = stmt.query_map(rusqlite::params![], Self::from_row)?
            .collect::<RusqliteResult<Vec<Self>>>()
            .map_err(|e| StorageError::QueryError(e.to_string()))?;
        Ok(scopes)
    }

    pub fn delete(conn: &Connection, id: i64) -> Result<usize> {
        let rows_affected = conn.execute("DELETE FROM scopes WHERE id = ?1", params![id])
            .map_err(|e| StorageError::QueryError(e.to_string()))?;
        Ok(rows_affected)
    }

    pub fn update_method(
        conn: &Connection,
        id: i64,
        method: CrawlMethod,
        keywords: Option<String>,
    ) -> Result<usize> {
        let rows_affected = conn.execute(
            "UPDATE scopes SET method = ?1, keywords = ?2 WHERE id = ?3",
            params![method.as_str(), keywords, id],
        )
        .map_err(|e| StorageError::QueryError(e.to_string()))?;
        Ok(rows_affected)
    }
}

impl Scope {
    // ... existing methods ...
    pub fn get_all(conn: &Connection) -> Result<Vec<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM scopes")?;
        let scopes = stmt.query_map(rusqlite::params![], |row| Scope::from_row(row))?
            .collect::<RusqliteResult<Vec<Self>>>()
            .map_err(|e| StorageError::QueryError(e.to_string()))?;
        Ok(scopes)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Page {
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
}

impl Page {
    pub fn from_row(row: &Row) -> RusqliteResult<Self> {
        Ok(Page {
            id: row.get("id")?,
            url: row.get("url")?,
            canonical_url: row.get("canonical_url")?,
            title: row.get("title")?,
            text_hash: row.get("text_hash")?,
            sim_hash: row.get("sim_hash")?,
            fetched_at: row.get("fetched_at")?,
            status_code: row.get("status_code")?,
            content_length: row.get("content_length")?,
            meta_json: row.get("meta_json")?,
            etag: row.get("etag")?,
            last_modified: row.get("last_modified")?,
            created_at: row.get("created_at")?,
        })
    }

    pub fn create(
        _storage: &crate::storage::Storage,
        url: &str,
        text_hash: Option<u64>,
        sim_hash: Option<u64>,
        title: Option<String>,
        canonical_url: Option<String>,
    ) -> Result<Self> {
        // For now, create a placeholder page
        // In a real implementation, this would use the storage pool
        Ok(Page {
            id: 1,
            url: url.to_string(),
            canonical_url,
            title,
            text_hash: text_hash.map(|h| h.to_string()),
            sim_hash: sim_hash.map(|h| h.to_string()),
            fetched_at: Utc::now(),
            status_code: Some(200),
            content_length: None,
            meta_json: None,
            etag: None,
            last_modified: None,
            created_at: Utc::now(),
        })
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Self> {
        Ok(conn.query_row("SELECT * FROM pages WHERE id = ?1", params![id], Self::from_row)
            .map_err(|_| StorageError::NotFound)?)
    }

    pub fn find_by_url(conn: &Connection, url: &str) -> Result<Self> {
        Ok(conn.query_row(
            "SELECT * FROM pages WHERE url = ?1",
            params![url],
            Self::from_row,
        )
        .map_err(|_| StorageError::NotFound)?)
    }

    pub fn find_by_text_hash(conn: &Connection, text_hash: &str) -> Result<Vec<Self>> {
        let mut stmt = conn.prepare("SELECT * FROM pages WHERE text_hash = ?1")?;
        let pages = stmt.query_map(params![text_hash], Self::from_row)?
            .collect::<RusqliteResult<Vec<Self>>>()
            .map_err(|e| StorageError::QueryError(e.to_string()))?;
        Ok(pages)
    }

    pub fn find_near_duplicates(conn: &Connection, sim_hash: &str) -> Result<Vec<Self>> {
        // This is a placeholder for a more efficient similarity search.
        // For now, we'll just check for an exact match.
        let mut stmt = conn.prepare("SELECT * FROM pages WHERE sim_hash = ?1")?;
        let pages = stmt.query_map(params![sim_hash], Self::from_row)?
            .collect::<RusqliteResult<Vec<Self>>>()
            .map_err(|e| StorageError::QueryError(e.to_string()))?;
        Ok(pages)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum StagedUrlStatus {
    Pending,
    Included,
    Excluded,
}

impl StagedUrlStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            StagedUrlStatus::Pending => "pending",
            StagedUrlStatus::Included => "included",
            StagedUrlStatus::Excluded => "excluded",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StagedUrl {
    pub id: Option<i64>,
    pub scope_id: i64,
    pub url: String,
    pub status: String,
    pub lastmod: Option<String>,
    pub priority: Option<f64>,
    pub discovered_at: DateTime<Utc>,
}

impl StagedUrl {
    pub fn from_row(row: &Row) -> RusqliteResult<Self> {
        Ok(StagedUrl {
            id: Some(row.get("id")?),
            scope_id: row.get("scope_id")?,
            url: row.get("url")?,
            status: row.get("status")?,
            lastmod: row.get("lastmod")?,
            priority: row.get("priority")?,
            discovered_at: row.get("discovered_at")?,
        })
    }

    pub fn create(
        conn: &Connection,
        scope_id: i64,
        url: &str,
        lastmod: Option<&str>,
    ) -> Result<Self> {
        let lastmod_dt = lastmod.and_then(|s| DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc)));
        conn.execute(
            "INSERT INTO staged_urls (scope_id, url, lastmod) VALUES (?1, ?2, ?3)",
            params![scope_id, url, lastmod_dt],
        )?;
        let id = conn.last_insert_rowid();
        Self::find_by_id(conn, id)
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Self> {
        Ok(conn.query_row(
            "SELECT * FROM staged_urls WHERE id = ?1",
            params![id],
            Self::from_row,
        )
        .map_err(|_| StorageError::NotFound)?)
    }

    pub fn set_status(&self, conn: &Connection, status: StagedUrlStatus) -> Result<()> {
        conn.execute(
            "UPDATE staged_urls SET status = ?1 WHERE id = ?2",
            params![status.as_str(), self.id],
        )?;
        Ok(())
    }

    pub fn set_priority(&self, conn: &Connection, priority: i32) -> Result<()> {
        conn.execute(
            "UPDATE staged_urls SET priority = ?1 WHERE id = ?2",
            params![priority, self.id],
        )?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EventLevel {
    Info,
    Warn,
    Error,
    Debug,
}

impl EventLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventLevel::Info => "INFO",
            EventLevel::Warn => "WARN",
            EventLevel::Error => "ERROR",
            EventLevel::Debug => "DEBUG",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Event {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub level: EventLevel,
    pub message: String,
    pub context: Option<String>,
}

impl Event {
    pub fn create(conn: &Connection, level: EventLevel, message: &str, context: Option<&str>) -> Result<Self> {
        conn.execute(
            "INSERT INTO events (level, message, context) VALUES (?1, ?2, ?3)",
            params![level.as_str(), message, context],
        )?;
        let id = conn.last_insert_rowid();
        Self::find_by_id(conn, id)
    }

    pub fn find_by_id(conn: &Connection, id: i64) -> Result<Self> {
        Ok(conn.query_row(
            "SELECT * FROM events WHERE id = ?1",
            params![id],
            |row| {
                let level_str: String = row.get("level")?;
                let level = match level_str.as_str() {
                    "INFO" => EventLevel::Info,
                    "WARN" => EventLevel::Warn,
                    "ERROR" => EventLevel::Error,
                    "DEBUG" => EventLevel::Debug,
                    _ => EventLevel::Info,
                };
                Ok(Event {
                    id: row.get("id")?,
                    timestamp: row.get("timestamp")?,
                    level,
                    message: row.get("message")?,
                    context: row.get("context")?,
                })
            },
        )
        .map_err(|_| StorageError::NotFound)?)
    }
}
