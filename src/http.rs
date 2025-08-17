use dashmap::DashMap;
use reqwest::header::{HeaderMap, ACCEPT_ENCODING, USER_AGENT};
use reqwest::{Client, Response, Result, Url};
use std::time::{Duration, Instant};
use tokio_retry::strategy::{ExponentialBackoff, jitter};
use tokio_retry::Retry;

const USER_AGENTS: [&str; 5] = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.3 Safari/605.1.15",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/108.0.0.0 Safari/537.36",
];

#[derive(Clone)]
struct HostState {
    last_request_at: Instant,
    ewma_rtt: Duration,
}

#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    host_states: DashMap<String, HostState>,
}

impl HttpClient {
    pub fn new(config: &crate::config::Config) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT_ENCODING, "br,gzip,deflate".parse().unwrap());

        let mut client_builder = Client::builder()
            .pool_max_idle_per_host(config.http.pool_max_idle_per_host)
            .connect_timeout(config.http.connect_timeout)
            .timeout(config.http.request_timeout)
            .default_headers(headers);

        if let Some(proxy_url) = &config.http.proxy {
            client_builder = client_builder.proxy(reqwest::Proxy::all(proxy_url)?);
        }

        let client = client_builder.build()?;

        Ok(HttpClient {
            client,
            host_states: DashMap::new(),
        })
    }

    pub fn get_client(&self) -> &Client {
        &self.client
    }

    pub fn get_random_user_agent(&self) -> &str {
        // Use a simple deterministic approach to avoid Send issues
        let index = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as usize % USER_AGENTS.len();
        &USER_AGENTS[index]
    }

    pub async fn pre_request_delay(&self, url: &Url) {
        let host = url.host_str().unwrap_or_default().to_string();

        let delay = if let Some(state) = self.host_states.get_mut(&host) {
            let time_since_last = state.last_request_at.elapsed();
            let base_delay = state.ewma_rtt * 2;
            // Use deterministic jitter based on host name to avoid Send issues
            let host_hash = host.len() % 1000;
            let jitter = Duration::from_millis(500 + host_hash as u64);
            let required_delay = base_delay + jitter;

            if time_since_last < required_delay {
                required_delay - time_since_last
            } else {
                Duration::from_secs(0)
            }
        } else {
            // First request to this host, use a default delay based on host
            let host_hash = host.len() % 5;
            Duration::from_secs(1 + host_hash as u64)
        };

        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }

    pub fn post_request_update(&self, url: &Url, rtt: Duration) {
        let host = url.host_str().unwrap_or_default().to_string();
        const ALPHA: f64 = 0.125;

        self.host_states
            .entry(host)
            .and_modify(|state| {
                state.last_request_at = Instant::now();
                let prev_rtt_micros = state.ewma_rtt.as_micros() as f64;
                let new_rtt_micros = rtt.as_micros() as f64;
                state.ewma_rtt = Duration::from_micros(
                    (prev_rtt_micros * (1.0 - ALPHA) + new_rtt_micros * ALPHA) as u64,
                );
            })
            .or_insert(HostState {
                last_request_at: Instant::now(),
                ewma_rtt: rtt,
            });
    }

    pub async fn get_with_retry(
        &self,
        storage: &crate::storage::Storage,
        url: &Url,
    ) -> Result<Response> {
        let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(3);

        let url_clone = url.clone();

        Retry::spawn(retry_strategy, || async {
            self.pre_request_delay(&url_clone).await;

            let start_time = Instant::now();
            let user_agent = self.get_random_user_agent();

            let page = storage.get_page_by_url(&url_clone.to_string()).await;

            let mut request = self.client.get(url_clone.clone());

            if let Ok(Some(page)) = page {
                if let Some(etag) = page.etag {
                    request = request.header("If-None-Match", etag);
                }
                if let Some(last_modified) = page.last_modified {
                    request = request.header("If-Modified-Since", last_modified);
                }
            }

            let response_result = request.header(USER_AGENT, user_agent).send().await;

            let rtt = start_time.elapsed();
            self.post_request_update(&url_clone, rtt);

            match response_result {
                Ok(response) => {
                    if response.status().is_server_error() {
                        let err = response.error_for_status().unwrap_err();
                        tracing::warn!("Server error for {}: {}. Retrying...", url_clone, err);
                        return Err(err);
                    }
                    Ok(response)
                }
                Err(err) => {
                    tracing::warn!("Request error for {}: {}. Retrying...", url_clone, err);
                    Err(err)
                }
            }
        })
        .await
    }
}