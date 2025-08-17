-- -----------------------------------------------------
-- Schema for Crawlify
-- -----------------------------------------------------

-- Enable WAL mode for better concurrency and performance.
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

-- -----------------------------------------------------
-- Table `scopes`
-- Stores the crawl configurations and patterns.
-- -----------------------------------------------------
CREATE TABLE IF NOT EXISTS scopes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  pattern TEXT NOT NULL UNIQUE,
  method TEXT NOT NULL DEFAULT 'DEFAULT' CHECK(method IN ('DEFAULT', 'NLP', 'HEADERS', 'CHANGED')),
  keywords TEXT, -- Comma-separated keywords for NLP mode
  is_active BOOLEAN NOT NULL DEFAULT 1,
  last_crawled_at DATETIME,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- -----------------------------------------------------
-- Table `staged_urls`
-- Stores URLs discovered from sitemaps before they are added to the frontier.
-- -----------------------------------------------------
CREATE TABLE IF NOT EXISTS staged_urls (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  scope_id INTEGER NOT NULL,
  url TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'included', 'excluded')),
  lastmod DATETIME,
  priority INTEGER NOT NULL DEFAULT 0,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (scope_id) REFERENCES scopes (id) ON DELETE CASCADE
);

-- -----------------------------------------------------
-- Table `frontier`
-- The main URL queue for the crawler.
-- -----------------------------------------------------
CREATE TABLE IF NOT EXISTS frontier (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  scope_id INTEGER NOT NULL,
  url TEXT NOT NULL UNIQUE,
  host TEXT NOT NULL,
  priority INTEGER NOT NULL DEFAULT 0,
  next_allowed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  tries INTEGER NOT NULL DEFAULT 0,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (scope_id) REFERENCES scopes (id) ON DELETE CASCADE
);

-- -----------------------------------------------------
-- Table `pages`
-- Stores the content and metadata of crawled pages.
-- -----------------------------------------------------
CREATE TABLE IF NOT EXISTS pages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  url TEXT NOT NULL UNIQUE,
  canonical_url TEXT,
  title TEXT,
  text_hash TEXT, -- XXH3 hash of the cleaned text content
  sim_hash TEXT, -- SimHash for near-duplicate detection
  fetched_at DATETIME NOT NULL,
  status_code INTEGER,
  content_length INTEGER,
  meta_json TEXT, -- JSON object for structured data (JSON-LD, OpenGraph)
  etag TEXT,
  last_modified TEXT,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- -----------------------------------------------------
-- Table `events`
-- Stores logging and event information for observability.
-- -----------------------------------------------------
CREATE TABLE IF NOT EXISTS events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  level TEXT NOT NULL CHECK(level IN ('INFO', 'WARN', 'ERROR', 'DEBUG')),
  message TEXT NOT NULL,
  context TEXT -- JSON object for additional context
);

-- -----------------------------------------------------
-- Indices for performance
-- -----------------------------------------------------
CREATE INDEX IF NOT EXISTS idx_scopes_is_active ON scopes(is_active);
CREATE INDEX IF NOT EXISTS idx_staged_urls_scope_id_status ON staged_urls(scope_id, status);
CREATE INDEX IF NOT EXISTS idx_frontier_scope_id_next_allowed_at ON frontier(scope_id, next_allowed_at);
DROP INDEX IF EXISTS idx_pages_url;
CREATE INDEX IF NOT EXISTS idx_pages_url_fetched_at ON pages(url, fetched_at);
CREATE INDEX IF NOT EXISTS idx_events_timestamp_level ON events(timestamp, level);

-- -----------------------------------------------------
-- Table `frontier_state`
-- Stores the serialized state of the frontier for crash recovery.
-- -----------------------------------------------------
CREATE TABLE IF NOT EXISTS frontier_state (
  id INTEGER PRIMARY KEY,
  state BLOB NOT NULL,
  saved_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
