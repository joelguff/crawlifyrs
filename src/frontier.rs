use crate::storage::models::Scope;
use anyhow::Result;
use dashmap::DashMap;
use priority_queue::PriorityQueue;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use url::Url;

const MAX_QUEUE_SIZE: usize = 1_000_000;

#[derive(Serialize, Deserialize, Clone)]
pub struct Frontier {
    host_queues: DashMap<String, HostQueue>,
    seen_urls: HashSet<Url>,
    size: usize,
}

#[derive(Serialize, Deserialize, Clone)]
struct HostQueue {
    queue: PriorityQueue<Url, i32>,
    #[serde(skip, default = "std::time::Instant::now")]
    next_allowed_at: Instant,
}

fn calculate_priority(url: &Url, _scope: &Scope, is_internal: bool, is_sitemap: bool) -> i32 {
    let mut priority = 0;

    if is_internal {
        priority += 5;
    }
    if is_sitemap {
        priority += 3;
    }
    // lastmod recency scoring will be implemented later
    // path length scoring
    let path_len = url.path().len();
    if path_len < 20 {
        priority += 1;
    }
    // parameter trap detection will be implemented later

    priority
}

impl Frontier {
    pub fn new() -> Self {
        Frontier {
            host_queues: DashMap::new(),
            seen_urls: HashSet::new(),
            size: 0,
        }
    }

    pub async fn save_state(&self, storage: &crate::storage::Storage) -> Result<()> {
        let state = bincode::serialize(self)?;
        storage.save_frontier_state(&state).await?;
        Ok(())
    }

    pub async fn load_state(storage: &crate::storage::Storage) -> Result<Option<Self>> {
        if let Some(state_data) = storage.load_frontier_state().await? {
            let frontier: Frontier = bincode::deserialize(&state_data)?;
            Ok(Some(frontier))
        } else {
            Ok(None)
        }
    }

    pub fn has_capacity(&self) -> bool {
        self.size < MAX_QUEUE_SIZE
    }

    pub fn add_url(&mut self, url: Url, scope: &Scope, is_sitemap: bool) -> Result<()> {
        if self.seen_urls.contains(&url) || !self.has_capacity() {
            return Ok(());
        }

        let host = url.host_str().unwrap_or_default().to_string();
        let is_internal = scope.pattern.contains(&host);
        let priority = calculate_priority(&url, scope, is_internal, is_sitemap);

        let mut host_queue = self
            .host_queues
            .entry(host)
            .or_insert_with(|| HostQueue {
                queue: PriorityQueue::new(),
                next_allowed_at: Instant::now(),
            });

        host_queue.queue.push(url.clone(), priority);
        self.seen_urls.insert(url);
        self.size += 1;

        Ok(())
    }

    pub fn get_next_url(&mut self) -> Option<Url> {
        let now = Instant::now();
        let mut best_host: Option<String> = None;
        let mut max_priority = i32::MIN;

        for entry in self.host_queues.iter() {
            if entry.value().next_allowed_at <= now {
                if let Some((_, &priority)) = entry.value().queue.peek() {
                    if priority > max_priority {
                        max_priority = priority;
                        best_host = Some(entry.key().clone());
                    }
                }
            }
        }

        if let Some(host) = best_host {
            let mut host_queue = self.host_queues.get_mut(&host).unwrap();
            host_queue.next_allowed_at = now + Duration::from_secs(1); // Politeness delay
            if let Some((url, _)) = host_queue.queue.pop() {
                self.size -= 1;
                Some(url)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}