use clap::Parser;
use crawlify::cli::Cli;
use crawlify::cli::Commands;
use crawlify::config::Config;
use crawlify::telemetry::{get_subscriber, init_subscriber};
use std::process;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = get_subscriber("crawlify".into(), "info".into(), || Box::new(std::io::stdout()));
    init_subscriber(subscriber);

    let config = Config::from_path("config.yaml")?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Crawl => {
            if let Err(e) = crawlify::commands::crawl(config).await {
                eprintln!("Application error: {}", e);
                process::exit(1);
            }
        }
        Commands::Init => {
            if let Err(e) = crawlify::commands::init(config).await {
                eprintln!("Error initializing database: {}", e);
                process::exit(1);
            }
        }
        Commands::Scopes => {
            if let Err(e) = crawlify::commands::list_scopes(config).await {
                eprintln!("Error listing scopes: {}", e);
                process::exit(1);
            }
        }
        Commands::Add { pattern } => {
            if let Err(e) = crawlify::commands::add_scope(config, pattern).await {
                eprintln!("Error adding scope: {}", e);
                process::exit(1);
            }
        }
        Commands::Remove { id } => {
            if let Err(e) = crawlify::commands::remove_scope(config, id).await {
                eprintln!("Error removing scope: {}", e);
                process::exit(1);
            }
        }
        Commands::Set { id, property, value } => {
            if let Err(e) = crawlify::commands::set_scope(config, id, property, value).await {
                eprintln!("Error setting scope property: {}", e);
                process::exit(1);
            }
        }
    }

    Ok(())
}
