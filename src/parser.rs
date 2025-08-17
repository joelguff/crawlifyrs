use html5ever::tokenizer::{BufferQueue, Token, Tokenizer, TokenizerOpts};
use readability::extractor;
use std::io::Read;
use url::Url;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OutlinkWithScore {
    pub url: String,
    pub nlp_score: Option<u8>, // 0 or 1, None if NLP not enabled
}

#[derive(Clone)]
pub struct PageData {
    pub title: Option<String>,
    pub canonical_url: Option<String>,
    pub outlinks: Vec<String>,
    pub outlinks_with_scores: Vec<OutlinkWithScore>, // New field for NLP scoring
    pub structured_data: serde_json::Value,
    pub main_content: String,
}

pub fn parse<R: Read>(mut input: R, url: &Url) -> PageData {
    let mut buffer = String::new();
    input.read_to_string(&mut buffer).unwrap();
    let tendril = html5ever::tendril::Tendril::from(buffer.clone());

    let main_content = if let Ok(url_str) = url.as_str().parse() {
        extractor::extract(&mut buffer.as_bytes(), &url_str)
    } else {
        // Fallback if URL conversion fails
        Ok(readability::extractor::Product {
            title: "".to_string(),
            content: "".to_string(),
            text: "".to_string()
        })
    }
        .map(|p| p.text)
        .unwrap_or_default();

    let sink = PageDataSink::new(url);
    let mut tokenizer = Tokenizer::new(
        sink,
        TokenizerOpts {
            profile: true,
            ..Default::default()
        },
    );

    let mut buffer_queue = BufferQueue::new();
    buffer_queue.push_back(tendril.try_reinterpret().unwrap());
    let _ = tokenizer.feed(&mut buffer_queue);
    tokenizer.end();

    let mut page_data = tokenizer.sink.get_page_data();
    page_data.main_content = main_content;
    page_data
}

use std::collections::{HashMap, HashSet};

struct PageDataSink<'a> {
    base_url: &'a Url,
    title: Option<String>,
    canonical_url: Option<Url>,
    outlinks: HashSet<Url>,
    structured_data: HashMap<String, Vec<String>>,
    in_title: bool,
    in_json_ld_script: bool,
    json_ld_content: String,
}

impl<'a> PageDataSink<'a> {
    fn new(base_url: &'a Url) -> Self {
        PageDataSink {
            base_url,
            title: None,
            canonical_url: None,
            outlinks: HashSet::new(),
            structured_data: HashMap::new(),
            in_title: false,
            in_json_ld_script: false,
            json_ld_content: String::new(),
        }
    }

    fn get_page_data(self) -> PageData {
        let mut structured_data_json = serde_json::Map::new();
        for (key, values) in self.structured_data {
            let json_values: Vec<serde_json::Value> =
                values.into_iter().map(serde_json::Value::String).collect();
            structured_data_json.insert(key, serde_json::Value::Array(json_values));
        }

        let outlinks_vec: Vec<String> = self.outlinks.iter().map(|u| u.to_string()).collect();
        let outlinks_with_scores: Vec<OutlinkWithScore> = outlinks_vec.iter()
            .map(|url| OutlinkWithScore {
                url: url.clone(),
                nlp_score: None, // Will be populated later by NLP processor
            })
            .collect();

        PageData {
            title: self.title,
            canonical_url: self.canonical_url.map(|u| u.to_string()),
            outlinks: outlinks_vec,
            outlinks_with_scores,
            structured_data: serde_json::Value::Object(structured_data_json),
            main_content: "".to_string(),
        }
    }
}

impl<'a> html5ever::tokenizer::TokenSink for PageDataSink<'a> {
    type Handle = ();

    fn process_token(
        &mut self,
        token: Token,
        _line_num: u64,
    ) -> html5ever::tokenizer::TokenSinkResult<()> {
        match token {
            Token::TagToken(tag) => {
                let tag_name = tag.name.as_ref();
                match tag.kind {
                    html5ever::tokenizer::TagKind::StartTag => {
                        if tag_name == "title" {
                            self.in_title = true;
                        } else if tag_name == "script" {
                            let is_json_ld = tag.attrs.iter().any(|attr| {
                                attr.name.local.as_ref() == "type"
                                    && attr.value.as_ref() == "application/ld+json"
                            });
                            if is_json_ld {
                                self.in_json_ld_script = true;
                            }
                        } else if tag_name == "meta" {
                            let mut property = None;
                            let mut content = None;
                            for attr in &tag.attrs {
                                if attr.name.local.as_ref() == "property" {
                                    property = Some(attr.value.to_string());
                                }
                                if attr.name.local.as_ref() == "content" {
                                    content = Some(attr.value.to_string());
                                }
                            }
                            if let (Some(prop), Some(cont)) = (property, content) {
                                self.structured_data
                                    .entry(prop)
                                    .or_insert_with(Vec::new)
                                    .push(cont);
                            }
                        } else if tag_name == "link" {
                            let mut rel = None;
                            let mut href = None;
                            for attr in &tag.attrs {
                                if attr.name.local.as_ref() == "rel" {
                                    rel = Some(attr.value.to_string());
                                }
                                if attr.name.local.as_ref() == "href" {
                                    href = Some(attr.value.to_string());
                                }
                            }
                            if let (Some(rel), Some(href)) = (rel, href) {
                                if rel == "canonical" {
                                    self.canonical_url = self.base_url.join(&href).ok();
                                }
                            }
                        } else if tag_name == "a" {
                            for attr in &tag.attrs {
                                if attr.name.local.as_ref() == "href" {
                                    if let Ok(abs_url) = self.base_url.join(&attr.value) {
                                        self.outlinks.insert(abs_url);
                                    }
                                }
                            }
                        }
                    }
                    html5ever::tokenizer::TagKind::EndTag => {
                        if tag_name == "title" {
                            self.in_title = false;
                        } else if tag_name == "script" && self.in_json_ld_script {
                            self.in_json_ld_script = false;
                            // End of script tag, attempt to parse
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&self.json_ld_content) {
                                self.structured_data
                                    .entry("json-ld".to_string())
                                    .or_insert_with(Vec::new)
                                    .push(serde_json::to_string(&json).unwrap_or_default());
                            }
                            self.json_ld_content.clear();
                        }
                    }
                }
            }
            Token::CharacterTokens(chars) => {
                if self.in_title {
                    self.title = Some(chars.to_string());
                } else if self.in_json_ld_script {
                    self.json_ld_content.push_str(&chars);
                }
            }
            _ => {}
        }
        html5ever::tokenizer::TokenSinkResult::Continue
    }
}