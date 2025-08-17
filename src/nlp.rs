use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use anyhow::Result;

pub struct KeywordMatcher {
    ac: AhoCorasick,
}

impl KeywordMatcher {
    pub fn new(keywords: &[String]) -> Result<Self> {
        let ac = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostFirst)
            .ascii_case_insensitive(true)
            .build(keywords)?;
        Ok(KeywordMatcher { ac })
    }

    pub fn find<'a>(&self, text: &'a str) -> bool {
        self.ac.is_match(text)
    }
}

pub struct NlpProcessor {
    keyword_matcher: Option<KeywordMatcher>,
}

impl NlpProcessor {
    pub fn new(config: &crate::config::NlpConfig) -> Result<Self> {
        // Log NLP configuration
        tracing::info!("NLP Set: {}", config.enabled);
        
        let keyword_matcher = if config.enabled {
            let keywords: Vec<String> = config
                .keywords
                .iter()
                .map(|s| s.trim().to_string())
                .collect();
            
            // Log keywords
            tracing::info!("Keywords: {}", keywords.join(", "));
            
            Some(KeywordMatcher::new(&keywords)?)
        } else {
            None
        };
        Ok(NlpProcessor { keyword_matcher })
    }

    pub fn is_match(&self, text: &str) -> bool {
        if let Some(matcher) = &self.keyword_matcher {
            tracing::info!("Checking for keywords....");
            let found = matcher.find(text);
            if found {
                tracing::info!("Keywords found");
            } else {
                tracing::info!("Keywords not found");
                tracing::info!("Skipping....");
            }
            found
        } else {
            true // If NLP is disabled, all content is considered a match
        }
    }

    /// Score individual outlinks based on their text content.
    /// Returns Some(1) if NLP is enabled and keywords match, Some(0) if no match, None if NLP disabled
    pub fn score_outlink(&self, outlink_text: &str) -> Option<u8> {
        if let Some(matcher) = &self.keyword_matcher {
            if matcher.find(outlink_text) {
                Some(1)
            } else {
                Some(0)
            }
        } else {
            None // NLP disabled, no scoring
        }
    }

    /// Score multiple outlinks and return updated vector with scores
    pub fn score_outlinks(&self, outlinks: &mut Vec<crate::parser::OutlinkWithScore>) {
        for outlink in outlinks.iter_mut() {
            outlink.nlp_score = self.score_outlink(&outlink.url);
        }
    }

    /// Check if NLP processing is enabled
    pub fn is_enabled(&self) -> bool {
        self.keyword_matcher.is_some()
    }

    /// Get the configured keywords for logging purposes
    pub fn get_keywords(&self) -> Vec<String> {
        // This is a bit hacky since we don't store the original keywords
        // In a real implementation, we'd store them separately
        Vec::new() // Placeholder - the keywords are logged during initialization
    }
}