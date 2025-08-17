use crate::error::CrawlifyError;
use bb8::Pool;
use bb8_rusqlite::RusqliteConnectionManager;

pub type DB = super::Storage;

impl DB {
    pub async fn migrate(&self) -> Result<(), CrawlifyError> {
        self.init().await
    }

    pub fn get_conn(&self) -> &Pool<RusqliteConnectionManager> {
        &self.pool
    }
}
