use crate::config::Config as CrawlerConfig;
use crate::deduplication::Deduplicator;
use crate::exporter::{JsonlExporter, Exporter, ExportPage};
use crate::pdf_exporter::PdfExporter;
use crate::frontier::Frontier;
use crate::http::HttpClient;
use crate::monitoring::{Metrics, Monitor};
use crate::nlp::NlpProcessor;
use crate::parser;
use crate::storage::connection::DB;
use crate::storage::models::EventLevel;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct Crawler {
    config: CrawlerConfig,
    db: DB,
    http_client: HttpClient,
    frontier: Frontier,
    exporter: JsonlExporter,
    pdf_exporter: PdfExporter,
    metrics: Arc<Metrics>,
}

impl Crawler {
    pub async fn new(config: CrawlerConfig) -> Result<Self> {
        let db = DB::new(&config.db_path).await?;
        db.migrate().await?;
        let http_client = HttpClient::new(&config)?;
        let frontier = if let Ok(Some(f)) = Frontier::load_state(&db).await {
            tracing::info!("Loaded frontier state from database");
            f
        } else {
            Frontier::new()
        };
        let exporter = JsonlExporter::new(&config.export_path).await?;
        let pdf_exporter = PdfExporter::new("crawled_pdfs")?;
        let metrics = Arc::new(Metrics::new());

        Ok(Crawler {
            config,
            db,
            http_client,
            frontier,
            exporter,
            pdf_exporter,
            metrics,
        })
    }

    #[allow(dead_code)]
    fn log_event(&self, level: EventLevel, message: &str, context: Option<&str>) {
        // For now, just log to tracing - Event::create needs proper connection handling
        match level {
            EventLevel::Info => tracing::info!("{}: {}", message, context.unwrap_or("")),
            EventLevel::Warn => tracing::warn!("{}: {}", message, context.unwrap_or("")),
            EventLevel::Error => tracing::error!("{}: {}", message, context.unwrap_or("")),
            EventLevel::Debug => tracing::debug!("{}: {}", message, context.unwrap_or("")),
        }
    }

    #[allow(dead_code)]
    fn log_performance_metrics(&self) {
        // This is a placeholder for logging performance metrics
        tracing::info!("Frontier depth: {}", self.frontier.size());
    }

    pub async fn run(&mut self, root_url: &str) -> Result<()> {
        // Extract all values from self at the very beginning to avoid lifetime issues
        let monitor = Monitor::new(self.metrics.clone());
        let db = self.db.clone();
        let metrics = self.metrics.clone();
        let http_client = Arc::new(self.http_client.clone());
        let exporter = Arc::new(self.exporter.clone());
        let pdf_exporter = Arc::new(self.pdf_exporter.clone());
        let nlp_processor = Arc::new(NlpProcessor::new(&self.config.nlp)?);
        let deduplicator = Arc::new(Deduplicator::new(self.db.clone()));
        let frontier = self.frontier.clone();
        let (global_concurrency, _) = self.config.get_concurrency();

        // Start monitoring in the background
        tokio::spawn(async move {
            monitor.run().await;
        });

        // Call helper function to avoid lifetime issues
        Self::run_crawler_loop(
            root_url,
            db,
            metrics,
            http_client,
            exporter,
            pdf_exporter,
            nlp_processor,
            deduplicator,
            frontier,
            global_concurrency,
        ).await
    }

    async fn run_crawler_loop(
        root_url: &str,
        db: crate::storage::connection::DB,
        metrics: Arc<Metrics>,
        http_client: Arc<HttpClient>,
        exporter: Arc<JsonlExporter>,
        pdf_exporter: Arc<PdfExporter>,
        nlp_processor: Arc<NlpProcessor>,
        deduplicator: Arc<Deduplicator>,
        frontier: Frontier,
        global_concurrency: usize,
    ) -> Result<()> {
        // Convert pattern to base URL by removing wildcards
        let base_url = if root_url.ends_with("/*") {
            &root_url[..root_url.len()-2]
        } else if root_url.ends_with("*") {
            &root_url[..root_url.len()-1]
        } else {
            root_url
        };
        let root_url = url::Url::parse(base_url)?;
        let scopes = Arc::new(db.get_active_scopes().await?);
        let frontier = Arc::new(tokio::sync::Mutex::new(frontier));

        // Note: Proper robots.txt parsing should be implemented here
        tracing::info!("Robots.txt parsing not implemented");

        let sitemap_fetcher =
            crate::sitemap::SitemapFetcher::new(&http_client, &db);
        let sitemaps = sitemap_fetcher.discover_sitemaps(&root_url).await?;
        for sitemap_url in sitemaps {
            sitemap_fetcher
                .parse_and_stage_sitemap(&sitemap_url)
                .await?;
        }
        sitemap_fetcher.process_staged_urls().await?;

        let mut frontier_guard = frontier.lock().await;
        if frontier_guard.is_empty() {
            // Get first scope as default - this should be improved
            if let Some(scope) = scopes.first() {
                frontier_guard.add_url(root_url.clone(), scope, false)?;
            }
        }
        drop(frontier_guard);

        // Phase 1: Discovery - Add and scan all outlinks first, collect URLs for phase 2
        tracing::info!("Phase 1: Starting outlink discovery phase");
        let discovered_urls = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let discovery_semaphore = Arc::new(Semaphore::new(global_concurrency));
        let mut discovery_handles = vec![];
        let mut discovery_empty_checks = 0;

        loop {
            let permit = discovery_semaphore.clone().acquire_owned().await?;
            let mut frontier_guard = frontier.lock().await;

            if let Some(url) = frontier_guard.get_next_url() {
                discovery_empty_checks = 0;
                drop(frontier_guard);
                
                let http_client = http_client.clone();
                let nlp_processor = nlp_processor.clone();
                let db = db.clone();
                let metrics = metrics.clone();
                let scopes_clone = scopes.clone();
                let frontier_clone = frontier.clone();
                let discovered_urls_clone = discovered_urls.clone();

                discovery_handles.push(tokio::spawn(async move {
                    metrics.inc_requests();
                    let response = http_client.get_with_retry(&db, &url).await;
                    metrics.add_bytes_in(response.as_ref().map(|r| r.content_length().unwrap_or(0)).unwrap_or(0));

                    let page_data = match response {
                        Ok(response) => {
                            if !response.status().is_success() {
                                return;
                            }
                            let text = response.text().await.unwrap_or_default();
                            parser::parse(text.as_bytes(), &url)
                        }
                        Err(_e) => {
                            return;
                        }
                    };

                    // Discovery phase: Always process and add ALL outlinks
                    let mut processed_page_data = page_data.clone();
                    nlp_processor.score_outlinks(&mut processed_page_data.outlinks_with_scores);
                    
                    let mut new_urls = Vec::new();
                    for outlink in &processed_page_data.outlinks {
                        if let Ok(outlink_url) = url::Url::parse(outlink) {
                            if let (Some(current_host), Some(outlink_host)) = (url.host_str(), outlink_url.host_str()) {
                                if current_host == outlink_host {
                                    new_urls.push(outlink_url.clone());
                                }
                            }
                        }
                    }
                    
                    // Add to discovered URLs for phase 2
                    {
                        let mut discovered_guard = discovered_urls_clone.lock().await;
                        discovered_guard.push(url.clone());
                        discovered_guard.extend(new_urls.iter().cloned());
                    }
                    
                    if !new_urls.is_empty() {
                        if let Some(scope) = scopes_clone.first() {
                            let frontier_for_urls = frontier_clone.clone();
                            let scope_for_urls = scope.clone();
                            tokio::spawn(async move {
                                let mut frontier_guard = frontier_for_urls.lock().await;
                                for new_url in new_urls {
                                    if let Err(e) = frontier_guard.add_url(new_url.clone(), &scope_for_urls, false) {
                                        tracing::warn!("Failed to add outlink {} to frontier: {}", new_url, e);
                                    } else {
                                        tracing::info!("Added outlink to frontier: {}", new_url);
                                    }
                                }
                            });
                        }
                    }
                    drop(permit);
                }));
            } else {
                drop(frontier_guard);
                drop(permit);
                
                if !discovery_handles.is_empty() {
                    while let Some(handle) = discovery_handles.pop() {
                        handle.await?;
                        break;
                    }
                    continue;
                }
                
                discovery_empty_checks += 1;
                if discovery_empty_checks < 3 {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    continue;
                }
                
                break;
            }
        }

        // Wait for all discovery tasks to complete
        for handle in discovery_handles {
            handle.await?;
        }

        let discovered_urls = Arc::try_unwrap(discovered_urls).unwrap().into_inner();
        let unique_urls: std::collections::HashSet<_> = discovered_urls.into_iter().collect();
        tracing::info!("Phase 1 complete: {} unique URLs discovered and ready for export", unique_urls.len());

        // Phase 2: Export - Process discovered URLs with NLP filtering
        tracing::info!("Phase 2: Starting export phase with NLP filtering");
        let export_semaphore = Arc::new(Semaphore::new(global_concurrency));
        let mut export_handles = vec![];

        for url in unique_urls {
            let permit = export_semaphore.clone().acquire_owned().await?;
            let http_client = http_client.clone();
            let exporter = exporter.clone();
            let pdf_exporter = pdf_exporter.clone();
            let nlp_processor = nlp_processor.clone();
            let deduplicator = deduplicator.clone();
            let db = db.clone();
            let metrics = metrics.clone();

            export_handles.push(tokio::spawn(async move {
                metrics.inc_requests();
                let response = http_client.get_with_retry(&db, &url).await;
                metrics.add_bytes_in(response.as_ref().map(|r| r.content_length().unwrap_or(0)).unwrap_or(0));

                let page_data = match response {
                    Ok(response) => {
                        if !response.status().is_success() {
                            drop(permit);
                            return;
                        }
                        let text = response.text().await.unwrap_or_default();
                        parser::parse(text.as_bytes(), &url)
                    }
                    Err(_e) => {
                        drop(permit);
                        return;
                    }
                };

                let mut processed_page_data = page_data.clone();
                nlp_processor.score_outlinks(&mut processed_page_data.outlinks_with_scores);

                // NLP filtering for export phase
                if !nlp_processor.is_match(&processed_page_data.main_content) {
                    drop(permit);
                    return;
                }

                let text_hash = crate::deduplication::text_hash(&processed_page_data.main_content);
                let sim_hash = crate::deduplication::sim_hash(&processed_page_data.main_content);
                let page = match crate::storage::models::Page::create(
                    &db,
                    url.as_str(),
                    Some(text_hash),
                    Some(sim_hash),
                    processed_page_data.title.clone(),
                    processed_page_data.canonical_url.clone(),
                ) {
                    Ok(p) => p,
                    Err(_e) => {
                        drop(permit);
                        return;
                    }
                };

                if deduplicator.is_duplicate(&page).await {
                    drop(permit);
                    return;
                }

                // Export with NLP filtering on outlinks
                let filtered_page_data = if nlp_processor.is_enabled() {
                    let filtered_outlinks: Vec<String> = processed_page_data.outlinks_with_scores
                        .iter()
                        .filter(|outlink| outlink.nlp_score == Some(1))
                        .map(|outlink| outlink.url.clone())
                        .collect();
                    
                    crate::parser::PageData {
                        title: processed_page_data.title.clone(),
                        canonical_url: processed_page_data.canonical_url.clone(),
                        outlinks: filtered_outlinks,
                        outlinks_with_scores: processed_page_data.outlinks_with_scores.clone(),
                        structured_data: processed_page_data.structured_data.clone(),
                        main_content: processed_page_data.main_content.clone(),
                    }
                } else {
                    processed_page_data.clone()
                };
                
                let base_export_page = crate::storage::models::Page {
                    id: page.id,
                    url: page.url.clone(),
                    canonical_url: page.canonical_url.clone(),
                    title: page.title.clone(),
                    text_hash: page.text_hash.clone(),
                    sim_hash: page.sim_hash.clone(),
                    fetched_at: page.fetched_at,
                    status_code: page.status_code,
                    content_length: page.content_length,
                    meta_json: Some(serde_json::to_string(&filtered_page_data.structured_data).unwrap_or_default()),
                    etag: page.etag.clone(),
                    last_modified: page.last_modified.clone(),
                    created_at: page.created_at,
                };

                // Create enhanced export page with outlinks_with_scores
                let mut enhanced_export_page = ExportPage::from(base_export_page.clone());
                enhanced_export_page.outlinks_with_scores = filtered_page_data.outlinks_with_scores.clone();
                
                if let Err(_e) = exporter.export_enhanced(&enhanced_export_page).await {
                    // Error logging
                }
                
                if let Ok(pdf_filename) = pdf_exporter.export_page_to_pdf(&base_export_page).await {
                    tracing::info!("Exporting as pdf name: {}", pdf_filename);
                } else {
                    tracing::error!("Failed to export PDF for {}", url);
                }
                
                tracing::info!("Exported page: {}", url);
                drop(permit);
            }));
        }

        // Wait for all export tasks to complete
        for handle in export_handles {
            handle.await?;
        }

        tracing::info!("Phase 2 complete: All matching pages exported");

        Ok(())
    }
}