use crate::error::CrawlifyError;
use bb8::Pool;
use bb8_rusqlite::RusqliteConnectionManager;
use models::{Page, Scope};

pub mod models;
pub mod connection;

#[derive(Clone)]
pub struct Storage {
    pub pool: Pool<RusqliteConnectionManager>,
}

impl Storage {
    pub async fn new(path: &str) -> Result<Self, CrawlifyError> {
        let manager = RusqliteConnectionManager::new(path);
        let pool = Pool::builder().build(manager).await?;
        Ok(Storage { pool })
    }

    pub async fn init(&self) -> Result<(), CrawlifyError> {
        let conn = self.pool.get().await?;
        let schema = include_str!("schema.sql");
        conn.execute_batch(schema)?;
        Ok(())
    }

    pub async fn get_active_scopes(&self) -> Result<Vec<Scope>, CrawlifyError> {
        let conn = self.pool.get().await?;
        let mut stmt = conn.prepare("SELECT * FROM scopes WHERE is_active = 1")?;
        let scopes = stmt
            .query_map(rusqlite::params![], |row| Scope::from_row(row))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(scopes)
    }

    pub async fn get_all_pages(&self) -> Result<Vec<Page>, CrawlifyError> {
        let conn = self.pool.get().await?;
        let mut stmt = conn.prepare("SELECT * FROM pages")?;
        let pages = stmt
            .query_map(rusqlite::params![], |row| Page::from_row(row))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(pages)
    }

    pub async fn get_page_by_url(&self, url: &str) -> Result<Option<Page>, CrawlifyError> {
        let conn = self.pool.get().await?;
        match conn.query_row("SELECT * FROM pages WHERE url = ?1", rusqlite::params![url], |row| Page::from_row(row)) {
            Ok(page) => Ok(Some(page)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn save_frontier_state(&self, _state: &[u8]) -> Result<(), CrawlifyError> {
        // Placeholder implementation
        Ok(())
    }

    pub async fn load_frontier_state(&self) -> Result<Option<Vec<u8>>, CrawlifyError> {
        // Placeholder implementation
        Ok(None)
    }

    pub async fn get_frontier(&self) -> Result<crate::frontier::Frontier, CrawlifyError> {
        // Placeholder implementation
        Ok(crate::frontier::Frontier::new())
    }

    pub async fn save_frontier(&self, _frontier: crate::frontier::Frontier) -> Result<(), CrawlifyError> {
        // Placeholder implementation
        Ok(())
    }

    pub async fn add_staged_url(&self, _url_entry: &crate::storage::models::StagedUrl) -> Result<(), CrawlifyError> {
        // Placeholder implementation
        Ok(())
    }

    pub async fn get_pending_staged_urls(&self) -> Result<Vec<crate::storage::models::StagedUrl>, CrawlifyError> {
        // Placeholder implementation
        Ok(vec![])
    }

    pub async fn update_staged_url_status(&self, _url: &str, _status: &str) -> Result<(), CrawlifyError> {
        // Placeholder implementation
        Ok(())
    }

    pub async fn find_pages_by_text_hash(&self, _text_hash: &str) -> Result<Vec<Page>, CrawlifyError> {
        // Placeholder implementation
        Ok(vec![])
    }

    pub async fn find_near_duplicates(&self, _sim_hash: &str) -> Result<Vec<Page>, CrawlifyError> {
        // Placeholder implementation
        Ok(vec![])
    }
}
