use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Initializes the database
    Init,
    /// Starts the crawler
    Crawl,
    /// Lists all crawl scopes
    Scopes,
    /// Adds a new crawl scope
    Add {
        /// URL pattern to add (e.g., https://example.com/*)
        pattern: String,
    },
    /// Removes a crawl scope by ID
    #[command(alias = "rm")]
    Remove {
        /// ID of the scope to remove
        id: i64,
    },
    /// Sets properties of a crawl scope
    Set {
        /// ID of the scope to modify
        id: i64,
        /// Property to set (method)
        property: String,
        /// Value to set
        value: String,
    },
}