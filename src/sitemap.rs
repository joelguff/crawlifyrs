use crate::http::HttpClient;
use crate::storage::models::StagedUrl;
use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;
use url::Url;

pub struct SitemapFetcher<'a> {
    http_client: &'a HttpClient,
    storage: &'a crate::storage::Storage,
}

#[derive(Debug)]
pub struct SitemapUrl {
    pub loc: Url,
    pub lastmod: Option<String>,
}

impl<'a> SitemapFetcher<'a> {
    pub fn new(http_client: &'a HttpClient, storage: &'a crate::storage::Storage) -> Self {
        SitemapFetcher {
            http_client,
            storage,
        }
    }

    pub async fn discover_sitemaps(&self, url: &Url) -> Result<Vec<Url>> {
        let mut sitemap_urls = Vec::new();
        let robots_txt_url = url.join("/robots.txt")?;

        // Note: A proper robots.txt parser should be used here.
        // For now, we'll just check for sitemap directives.
        if let Ok(resp) = self.http_client.get_client().get(robots_txt_url).send().await {
            if resp.status().is_success() {
                let text = resp.text().await?;
                for line in text.lines() {
                    if line.to_lowercase().starts_with("sitemap:") {
                        if let Ok(sitemap_url) = Url::parse(line.split_at(8).1.trim()) {
                            sitemap_urls.push(sitemap_url);
                        }
                    }
                }
            }
        }

        // If no sitemaps are found in robots.txt, check for a default sitemap.xml
        if sitemap_urls.is_empty() {
            let sitemap_xml_url = url.join("/sitemap.xml")?;
            if self
                .http_client
                .get_client()
                .head(sitemap_xml_url.clone())
                .send()
                .await?
                .status()
                .is_success()
            {
                sitemap_urls.push(sitemap_xml_url);
            }
        }

        Ok(sitemap_urls)
    }

    pub async fn parse_and_stage_sitemap(&self, url: &Url) -> Result<()> {
        let text = self.http_client.get_client().get(url.as_str()).send().await?.text().await?;
        let mut reader = Reader::from_str(&text);
        reader.config_mut().trim_text(true);
        let mut in_loc = false;
        let mut in_lastmod = false;
        let mut current_url: Option<SitemapUrl> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => match e.name().as_ref() {
                    b"url" => {
                        current_url = Some(SitemapUrl {
                            loc: Url::parse("https://example.com")?, // Placeholder
                            lastmod: None,
                        })
                    }
                    b"loc" => in_loc = true,
                    b"lastmod" => in_lastmod = true,
                    _ => (),
                },
                Ok(Event::Text(e)) => {
                    if let Some(url_entry) = current_url.as_mut() {
                        if in_loc {
                            if let Ok(parsed_url) = Url::parse(&e.unescape().unwrap_or_default()) {
                                url_entry.loc = parsed_url;
                            }
                            in_loc = false;
                        } else if in_lastmod {
                            url_entry.lastmod = Some(e.unescape().unwrap_or_default().into_owned());
                            in_lastmod = false;
                        }
                    }
                }
                Ok(Event::End(ref e)) if e.name().as_ref() == b"url" => {
                    if let Some(url_entry) = current_url.take() {
                        // Convert SitemapUrl to StagedUrl
                        let staged_url = StagedUrl {
                            id: None,
                            scope_id: 1, // Default scope
                            url: url_entry.loc.to_string(),
                            status: "pending".to_string(),
                            lastmod: url_entry.lastmod,
                            priority: Some(1.0),
                            discovered_at: chrono::Utc::now(),
                        };
                        self.storage.add_staged_url(&staged_url).await?;
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(e.into()),
                _ => (),
            }
        }

        Ok(())
    }

    pub async fn process_staged_urls(&self) -> Result<()> {
        let frontier = self.storage.get_frontier().await?;
        let staged_urls = self.storage.get_pending_staged_urls().await?;

        for staged_url in staged_urls {
            let _priority = if let Some(lastmod_str) = &staged_url.lastmod {
                // Try to parse the lastmod as a DateTime
                if let Ok(lastmod) = chrono::DateTime::parse_from_rfc3339(lastmod_str) {
                    let now = chrono::Utc::now();
                    let fourteen_days_ago = now - chrono::Duration::days(14);
                    if lastmod.with_timezone(&chrono::Utc) > fourteen_days_ago {
                        2.0
                    } else {
                        1.0
                    }
                } else {
                    1.0
                }
            } else {
                1.0
            };
            // For now, skip adding to frontier since we need scope info
            // This would need proper scope resolution
            self.storage
                .update_staged_url_status(&staged_url.url, "processed")
                .await?;
        }

        self.storage.save_frontier(frontier).await?;

        Ok(())
    }
}