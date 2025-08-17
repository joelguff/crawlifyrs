# 🕷️ Crawlify - The Smart Web Crawler That Actually Works

> From 64 compilation errors to crawling the entire web – this is the comeback story you didn't know you needed.

## What's This All About?

Hey there! Meet **Crawlify** – a Rust-powered web crawler that's been through some serious character development. What started as a completely broken project with more errors than a student's first code submission has evolved into a pretty slick web crawling machine.

Here's what makes Crawlify special:

- 🧠 **Smart NLP Filtering** - Only grabs pages that actually contain the keywords you care about
- 📄 **PDF Export** - Saves beautiful PDFs of web pages using headless Chrome
- 🔗 **Full Website Crawling** - Follows links and crawls entire websites, not just single pages
- ⚡ **Async Everything** - Built on Tokio for blazing fast concurrent crawling
- 🎯 **Deduplication** - Smart enough not to crawl the same content twice
- 📊 **Rich Metadata** - Extracts titles, hashes, and structured data

## The Comeback Story

This project was... let's say "challenging" when we first met. Picture this:
- **64 compilation errors** 💥
- **28 warnings** screaming for attention  
- Dependencies fighting each other like siblings
- Code that looked like it was written during a caffeine crash

But hey, that's what made it fun! We systematically fixed every single issue, added some killer features, and now it's actually useful. Think of it as the ultimate debugging bootcamp.

## Quick Start (The "I Just Want It To Work" Guide)

### 1. Build the Beast

```bash
git clone <your-repo-url>
cd crawlify
cargo build --release
```

### 2. Set It Up

First, initialize the database:
```bash
./target/release/crawlify init
```

### 3. Add a Website to Crawl

```bash
./target/release/crawlify add "https://example.com/*"
```

### 4. Configure Your Keywords

Edit `config.yaml` to tell Crawlify what you're looking for:

```yaml
nlp:
  enabled: true
  keywords:
    - "rust"
    - "programming" 
    - "web development"
    - "your important keywords here"
```

### 5. Let It Rip!

```bash
./target/release/crawlify crawl
```

Sit back and watch as Crawlify:
- Discovers and follows links across the website
- Filters pages based on your keywords
- Generates beautiful PDFs in the `crawled_pdfs/` folder
- Saves metadata in `crawled_data.jsonl`

## Real-World Example

We tested this bad boy on the Elixir website (because why not?). Here's what happened:

```
🎯 Crawled: 39 pages
📄 Generated: 32 PDFs  
⏱️  Time: 2+ minutes of intensive crawling
💾 Output: Everything from 195KB to 5.3MB PDFs
```

Pages it grabbed included documentation, blog posts, release notes, and learning resources – all because they contained our Elixir-related keywords.

## Configuration Deep Dive

### The Main Config (`config.yaml`)

```yaml
# Database and output paths
db_path: "crawlify.db"
export_path: "crawled_data.jsonl"

# HTTP settings (because websites can be picky)
http:
  connect_timeout: "30s"
  request_timeout: "60s"
  pool_max_idle_per_host: 10

# The magic NLP filtering
nlp:
  enabled: true
  keywords:
    - "your"
    - "important" 
    - "keywords"
    - "here"
```

### CLI Commands You'll Actually Use

```bash
# Initialize everything
crawlify init

# Add a website to crawl  
crawlify add "https://example.com/*"

# Start crawling (the fun part)
crawlify crawl

# See what you've collected
crawlify list
```

## What You Get

### PDF Archives 📄
Beautiful, high-fidelity PDFs of every page that matches your keywords. Perfect for:
- Research and documentation
- Offline reading
- Presentations and reports
- "I swear the website said that yesterday" moments

### Structured Metadata 📊
JSON Lines format with all the good stuff:
- URLs and titles
- Content hashes for deduplication  
- Timestamps and status codes
- Canonical URLs and metadata

### Smart Filtering 🧠
Only processes pages that actually contain your keywords. No more wading through irrelevant content.

## The Technical Bits (For the Curious)

- **Language**: Rust (because we like our crawlers fast and safe)
- **Async Runtime**: Tokio (for that sweet, sweet concurrency)
- **PDF Generation**: Headless Chrome (because it just works)
- **NLP Filtering**: Aho-Corasick algorithm (fancy pattern matching)
- **Storage**: SQLite (simple and reliable)
- **Deduplication**: SimHash and text hashing (no duplicate content)

## What We Fixed (The Redemption Arc)

This project went from "completely broken" to "actually useful" through:

1. **Dependency Hell → Harmony**: Resolved version conflicts across 20+ crates
2. **Compilation Chaos → Clean Code**: Fixed 64 compilation errors 
3. **Warning Nightmare → Warning-Free**: Eliminated all 28 warnings
4. **Single Page → Full Website**: Added intelligent link following
5. **Raw Data → PDF Beauty**: Integrated headless Chrome for PDF export
6. **Everything → Smart Filtering**: Added NLP keyword filtering

## Contributing

Found a bug? Want to add a feature? PRs welcome! Just remember:
- Run `cargo test` (when we add tests... that's next!)
- Keep the conversational README spirit alive
- Add your feature to the "what we fixed" list if it's cool enough

## License

Whatever makes lawyers happy. Use it, modify it, just don't blame us if it crawls the entire internet.

---

**Happy Crawling!** 🕷️✨

*P.S. - If you find any bugs, remember: we started with 64 compilation errors. A few runtime issues are practically a feature at this point.*