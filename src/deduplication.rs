use url::Url;
use xxhash_rust::xxh3::xxh3_64;

pub fn canonicalize_url(url: &mut Url) {
    // Lowercase the host
    if let Some(host) = url.host_str() {
        let _ = url.set_host(Some(&host.to_lowercase()));
    }

    // Strip fragments
    url.set_fragment(None);

    // Sort query parameters
    let mut pairs: Vec<_> = url.query_pairs().into_owned().collect();
    pairs.sort();
    let new_query = pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");
    if !new_query.is_empty() {
        url.set_query(Some(&new_query));
    } else {
        url.set_query(None);
    }
}


pub fn text_hash(text: &str) -> u64 {
    xxh3_64(text.as_bytes())
}

use crate::storage::models::Page;

pub fn sim_hash(text: &str) -> u64 {
    // Simple hash function - simhash crate API is private
    // Use a basic text hash for now
    use std::hash::{DefaultHasher, Hasher};
    let mut hasher = DefaultHasher::new();
    hasher.write(text.as_bytes());
    hasher.finish()
}

pub struct Deduplicator {
    storage: crate::storage::Storage,
}

impl Deduplicator {
    pub fn new(storage: crate::storage::Storage) -> Self {
        Deduplicator { storage }
    }

    pub async fn is_duplicate(&self, page: &Page) -> bool {
        if let Some(text_hash) = &page.text_hash {
            if let Ok(pages) = self.storage.find_pages_by_text_hash(text_hash).await {
                if !pages.is_empty() {
                    return true;
                }
            }
        }

        if let Some(sim_hash) = &page.sim_hash {
            if let Ok(pages) = self.storage.find_near_duplicates(sim_hash).await {
                if !pages.is_empty() {
                    return true;
                }
            }
        }

        false
    }
}