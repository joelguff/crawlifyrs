Crawlify is a Rust-powered web crawler.

Features:

- NLP Filtering - Only grabs pages that actually contain the keywords
- PDF Export - Saves PDFs of web pages using headless Chrome
- Full Website Crawling - with scopes follows links and crawls entire websites, not just single pages.
- Deduplication - Smart enough not to crawl the same content twice
- Rich Metadata - Extracts titles, hashes, and structured data

1. Do this:

```bash
git clone https://github.com/joelguff/crawlify.git
cd crawlify
cargo build --release
```

2. Set It Up

```bash
cd /target/release/
.\crawlify.exe init <- creates db
```

3. Add a Website to Crawl

```bash
.\crawlify.exe add "https://example.com/*"
```
Scoping:

* = Wildcard.

### 4. Configure Your Keywords

Edit `config.yaml` to tell Crawlify what you're looking for:

```yaml
# Crawlify Configuration
db_path: "crawlify.db"
export_path: "crawled_data.jsonl"

http:
  connect_timeout: "30s"
  request_timeout: "60s"
  pool_max_idle_per_host: 10
  proxy: null

nlp:
  enabled: true
  keywords:
    - "Example1"
    - "Example2"
```

5. Begin:

```bash
.\crawlify crawl
```

Crawlify:
- Discovers and follows links across the website
- Filters pages based on your keywords
- Generates PDFs in the `crawled_pdfs/` folder
- Saves metadata in `crawled_data.jsonl`


CLI Commands

```bash
crawlify init
 
crawlify add "https://example.com/*"

crawlify scopes <- lists all websites

1. https://example.com/*

crawlify rm 1 <- removes https://example.com/*

crawlify crawl

```

This is a FOSS software, use it edit and contribute, help me out also join: discord:
https://discord.gg/bX5tfjBN