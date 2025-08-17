use anyhow::Result;
use headless_chrome::{Browser, LaunchOptions};
use std::path::Path;
use std::fs;
use std::{thread, time::Duration};
use crate::storage::models::Page;

pub struct PdfExporter {
    output_dir: String,
}

impl PdfExporter {
    pub fn new(output_dir: &str) -> Result<Self> {
        // Create output directory if it doesn't exist
        fs::create_dir_all(output_dir)?;
        Ok(PdfExporter {
            output_dir: output_dir.to_string(),
        })
    }

    pub async fn export_page_to_pdf(&self, page: &Page) -> Result<String> {
        let browser = Browser::new(LaunchOptions::default_builder()
            .headless(true)
            .build()
            .expect("Could not find chrome-executable"))?;

        let tab = browser.new_tab()?;
        
        // Parse URL to check for fragment
        let parsed_url = url::Url::parse(&page.url)?;
        
        // Navigate to the URL
        tab.navigate_to(&page.url)?;
        
        // Wait for the page to load
        tab.wait_until_navigated()?;
        
        // If there's a fragment (hash), extract only that section
        if let Some(fragment) = parsed_url.fragment() {
            // Wait a bit more for dynamic content to load
            thread::sleep(Duration::from_millis(1500));
            
            // Try to find and extract the specific section content
            let section_script = format!(
                r#"
                (function() {{
                    // Find the target heading/element
                    let targetElement = document.getElementById('{}');
                    
                    // Fallback searches
                    if (!targetElement) {{
                        targetElement = document.querySelector('a[name="{}"]');
                    }}
                    
                    if (!targetElement) {{
                        const headings = document.querySelectorAll('h1, h2, h3, h4, h5, h6');
                        for (const heading of headings) {{
                            const headingText = heading.textContent.toLowerCase().trim();
                            const fragmentText = '{}'. replace(/[-_]/g, ' ').toLowerCase();
                            if (headingText.includes(fragmentText) ||
                                heading.id === '{}' ||
                                headingText.replace(/\s+/g, '-') === '{}') {{
                                targetElement = heading;
                                break;
                            }}
                        }}
                    }}
                    
                    if (!targetElement) {{
                        return false;
                    }}
                    
                    // Collect the section content
                    let sectionContent = [];
                    
                    // Start with the target element (usually a heading)
                    sectionContent.push(targetElement.cloneNode(true));
                    
                    // Collect all following siblings until we hit another heading of same or higher level
                    let targetLevel = 6; // default to h6 level
                    if (targetElement.tagName.match(/^H[1-6]$/)) {{
                        targetLevel = parseInt(targetElement.tagName.substring(1));
                    }}
                    
                    let currentElement = targetElement.nextElementSibling;
                    while (currentElement) {{
                        // Stop if we hit another heading of same or higher level
                        if (currentElement.tagName.match(/^H[1-6]$/)) {{
                            const currentLevel = parseInt(currentElement.tagName.substring(1));
                            if (currentLevel <= targetLevel) {{
                                break;
                            }}
                        }}
                        
                        // Add this element to our section content
                        sectionContent.push(currentElement.cloneNode(true));
                        currentElement = currentElement.nextElementSibling;
                    }}
                    
                    // Create a new page with just the section content
                    const newBody = document.createElement('div');
                    newBody.style.cssText = `
                        font-family: system-ui, -apple-system, sans-serif;
                        max-width: 800px;
                        margin: 20px auto;
                        padding: 20px;
                        line-height: 1.6;
                    `;
                    
                    // Add title
                    const title = document.createElement('h1');
                    title.textContent = 'Section: ' + targetElement.textContent.replace('#', '').trim();
                    title.style.cssText = `
                        color: #007bff;
                        border-bottom: 2px solid #007bff;
                        padding-bottom: 10px;
                        margin-bottom: 20px;
                    `;
                    newBody.appendChild(title);
                    
                    // Add the collected section content
                    const contentDiv = document.createElement('div');
                    contentDiv.style.cssText = `
                        background: #f8f9fa;
                        border: 2px solid #007bff;
                        border-radius: 5px;
                        padding: 20px;
                        margin: 10px 0;
                    `;
                    
                    sectionContent.forEach(element => {{
                        contentDiv.appendChild(element);
                    }});
                    
                    newBody.appendChild(contentDiv);
                    
                    // Add URL info
                    const urlInfo = document.createElement('div');
                    urlInfo.textContent = 'Source: ' + window.location.href;
                    urlInfo.style.cssText = `
                        margin-top: 20px;
                        padding: 10px;
                        background: #e9ecef;
                        border-radius: 3px;
                        font-size: 12px;
                        color: #6c757d;
                    `;
                    newBody.appendChild(urlInfo);
                    
                    // Replace the body content
                    document.body.innerHTML = '';
                    document.body.appendChild(newBody);
                    
                    // Scroll to top
                    window.scrollTo(0, 0);
                    
                    return true;
                }})();
                "#,
                fragment, fragment, fragment.replace('-', " "), fragment, fragment
            );
            
            match tab.evaluate(&section_script, false) {
                Ok(result) => {
                    thread::sleep(Duration::from_millis(1000));
                    tracing::info!("Extracted section content for fragment: #{}", fragment);
                    if let Some(success) = result.value.unwrap().as_bool() {
                        if !success {
                            tracing::warn!("Could not find section for fragment: #{}", fragment);
                        }
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to extract section for fragment #{}: {}", fragment, e);
                }
            }
        }
        
        // Generate a safe filename from the URL
        let filename = self.generate_filename(&page.url, page.id);
        let output_path = Path::new(&self.output_dir).join(&filename);
        
        // Generate PDF
        let pdf_data = tab.print_to_pdf(None)?;
        fs::write(&output_path, pdf_data)?;
        
        tracing::info!("Exported PDF: {}", output_path.display());
        Ok(filename)
    }

    fn generate_filename(&self, url: &str, id: i64) -> String {
        // Create a safe filename from URL and ID, including fragment
        let parsed_url = url::Url::parse(url).unwrap_or_else(|_| {
            url::Url::parse("http://example.com/page").unwrap()
        });
        
        let mut url_parts = parsed_url.path().to_string();
        
        // Include fragment (hash part) in filename if present
        if let Some(fragment) = parsed_url.fragment() {
            url_parts.push('_');
            url_parts.push_str(fragment);
        }
        
        let safe_name = url_parts
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>()
            .trim_matches('_')
            .to_string();
        
        let name = if safe_name.is_empty() {
            "homepage".to_string()
        } else {
            safe_name
        };
        
        format!("{}_{}.pdf", name, id)
    }
}

impl Clone for PdfExporter {
    fn clone(&self) -> Self {
        PdfExporter {
            output_dir: self.output_dir.clone(),
        }
    }
}