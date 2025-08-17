use crate::config::Config;
use crate::crawler::Crawler;
use crate::error::CrawlifyError;
use crate::storage::Storage;
use comfy_table::{Cell, Table};
use tokio::time::Instant;

pub async fn init(config: Config) -> Result<(), CrawlifyError> {
    let storage = Storage::new(&config.db_path).await?;
    storage.init().await?;
    println!("Database initialized at {}", config.db_path);
    Ok(())
}

pub async fn crawl(config: Config) -> Result<(), CrawlifyError> {
    let storage = Storage::new(&config.db_path).await?;
    let scopes = storage.get_active_scopes().await?;
    if scopes.is_empty() {
        println!("No active scopes found. Please add a scope first.");
        return Ok(());
    }

    let mut crawler = Crawler::new(config).await?;
    let start = Instant::now();
    for scope in scopes {
        crawler.run(&scope.pattern).await?;
    }
    let duration = start.elapsed();
    println!("Crawl finished in {}", humantime::format_duration(duration));
    Ok(())
}

pub async fn list(config: Config) -> Result<(), CrawlifyError> {
    let storage = Storage::new(&config.db_path).await?;
    let pages = storage.get_all_pages().await?;
    let mut table = Table::new();
    table.set_header(vec![
        "ID",
        "URL",
        "Title",
        "Crawled At",
        "Status",
        "Content Hash",
    ]);

    for page in pages {
        table.add_row(vec![
            Cell::new(page.id),
            Cell::new(page.url),
            Cell::new(page.title.unwrap_or_default()),
            Cell::new(page.created_at.to_rfc2822()),
            Cell::new(page.status_code.unwrap_or(0)),
            Cell::new(
                page.text_hash
                    .map(|h| hex::encode(h))
                    .unwrap_or_default(),
            ),
        ]);
    }

    println!("{table}");

    Ok(())
}

pub async fn add_scope(config: Config, pattern: String) -> Result<(), CrawlifyError> {
    let storage = Storage::new(&config.db_path).await?;
    let conn = storage.pool.get().await?;
    
    // Determine method based on config
    let method = if config.nlp.enabled {
        "NLP"
    } else {
        "DEFAULT"
    };
    
    conn.execute(
        "INSERT INTO scopes (pattern, method, is_active) VALUES (?1, ?2, 1)",
        rusqlite::params![pattern, method],
    )?;
    
    println!("Added scope: {} (Method: {})", pattern, method);
    Ok(())
}

pub async fn list_scopes(config: Config) -> Result<(), CrawlifyError> {
    let storage = Storage::new(&config.db_path).await?;
    let conn = storage.pool.get().await?;
    
    let mut stmt = conn.prepare(
        "SELECT id, pattern, method, is_active, created_at FROM scopes ORDER BY id"
    )?;
    
    let scope_rows = stmt.query_map(rusqlite::NO_PARAMS, |row| {
        Ok((
            row.get::<_, i64>(0)?,      // id
            row.get::<_, String>(1)?,   // pattern
            row.get::<_, String>(2)?,   // method
            row.get::<_, bool>(3)?,     // is_active
            row.get::<_, String>(4)?,   // created_at
        ))
    })?;
    
    let mut table = Table::new();
    table.set_header(vec![
        "ID",
        "Pattern",
        "Method",
        "Active",
        "Created At",
    ]);
    
    for scope_result in scope_rows {
        let (id, pattern, method, is_active, created_at) = scope_result?;
        table.add_row(vec![
            Cell::new(id),
            Cell::new(pattern),
            Cell::new(method),
            Cell::new(if is_active { "Yes" } else { "No" }),
            Cell::new(created_at),
        ]);
    }
    
    println!("{table}");
    Ok(())
}

pub async fn set_scope(config: Config, id: i64, property: String, value: String) -> Result<(), CrawlifyError> {
    let storage = Storage::new(&config.db_path).await?;
    let conn = storage.pool.get().await?;
    
    match property.to_lowercase().as_str() {
        "method" => {
            // Validate the method value
            let valid_methods = ["DEFAULT", "NLP", "HEADERS", "CHANGED"];
            let method_upper = value.to_uppercase();
            
            if !valid_methods.contains(&method_upper.as_str()) {
                println!("Invalid method '{}'. Valid methods: {}", value, valid_methods.join(", "));
                return Ok(());
            }
            
            // Check if scope exists first
            let scope_info: Result<(String,), rusqlite::Error> = conn.query_row(
                "SELECT pattern FROM scopes WHERE id = ?1",
                rusqlite::params![id],
                |row| Ok((row.get::<_, String>(0)?,))
            );
            
            match scope_info {
                Ok((pattern,)) => {
                    // Update the method
                    let rows_affected = conn.execute(
                        "UPDATE scopes SET method = ?1 WHERE id = ?2",
                        rusqlite::params![method_upper, id],
                    )?;
                    
                    if rows_affected > 0 {
                        println!("Updated scope {} method to '{}': {}", id, method_upper, pattern);
                    } else {
                        println!("No scope found with ID: {}", id);
                    }
                }
                Err(_) => {
                    println!("No scope found with ID: {}", id);
                }
            }
        }
        _ => {
            println!("Unknown property '{}'. Supported properties: method", property);
        }
    }
    
    Ok(())
}

pub async fn remove_scope(config: Config, id: i64) -> Result<(), CrawlifyError> {
    let storage = Storage::new(&config.db_path).await?;
    let conn = storage.pool.get().await?;
    
    // First, check if the scope exists and get its pattern for confirmation
    let scope_info: Result<(String,), rusqlite::Error> = conn.query_row(
        "SELECT pattern FROM scopes WHERE id = ?1",
        rusqlite::params![id],
        |row| Ok((row.get::<_, String>(0)?,))
    );
    
    match scope_info {
        Ok((pattern,)) => {
            // Delete the scope
            let rows_affected = conn.execute(
                "DELETE FROM scopes WHERE id = ?1",
                rusqlite::params![id],
            )?;
            
            if rows_affected > 0 {
                println!("Removed scope {} (ID: {}): {}", id, id, pattern);
            } else {
                println!("No scope found with ID: {}", id);
            }
        }
        Err(_) => {
            println!("No scope found with ID: {}", id);
        }
    }
    
    Ok(())
}