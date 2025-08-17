use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

#[derive(Default)]
pub struct Metrics {
    pub requests_total: AtomicU64,
    pub bytes_in_total: AtomicU64,
    pub host_backoffs: AtomicU64,
    pub frontier_depth: AtomicU64,
    pub mem_rss_mb: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn inc_requests(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_bytes_in(&self, bytes: u64) {
        self.bytes_in_total.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn inc_host_backoffs(&self) {
        self.host_backoffs.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_frontier_depth(&self, depth: u64) {
        self.frontier_depth.store(depth, Ordering::Relaxed);
    }

    pub fn set_mem_rss(&self, mem_mb: u64) {
        self.mem_rss_mb.store(mem_mb, Ordering::Relaxed);
    }
}

pub struct Monitor {
    metrics: Arc<Metrics>,
}

impl Monitor {
    pub fn new(metrics: Arc<Metrics>) -> Self {
        Monitor { metrics }
    }

    pub async fn run(&self) {
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            self.log_metrics();
        }
    }

    fn log_metrics(&self) {
        let requests = self.metrics.requests_total.load(Ordering::Relaxed);
        let bytes = self.metrics.bytes_in_total.load(Ordering::Relaxed);
        let backoffs = self.metrics.host_backoffs.load(Ordering::Relaxed);
        let depth = self.metrics.frontier_depth.load(Ordering::Relaxed);
        let mem = self.metrics.mem_rss_mb.load(Ordering::Relaxed);

        tracing::info!(
            requests,
            bytes_in = bytes,
            host_backoffs = backoffs,
            frontier_depth = depth,
            mem_rss_mb = mem,
            "Crawl Stats"
        );
    }
}