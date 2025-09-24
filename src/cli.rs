use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use feed_rs::parser;
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;
use crate::Config;

// Time constants for timeouts
const REQUEST_TIMEOUT_SECS: u64 = 30;
const OPERATION_TIMEOUT_SECS: u64 = 45;

#[derive(Parser)]
#[command(name = "yomi")]
#[command(about = "A beautiful terminal RSS reader with Tokyo Night theme", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the interactive TUI reader
    #[command(alias = "tui")]
    Read,
    
    /// Add a new feed
    Add {
        /// URL of the RSS/Atom feed to add
        url: String,
        
        /// Custom name for the feed (optional)
        #[arg(short, long)]
        name: Option<String>,
    },
    
    /// Remove a feed by name or index
    Remove {
        /// Name or index of the feed to remove
        target: String,
    },
    
    /// List all configured feeds
    List,
    
    /// Refresh feeds and show latest items
    Refresh {
        /// Name of a specific feed to refresh (optional)
        #[arg(short, long)]
        feed: Option<String>,
    },
}

/// Validate a feed URL by fetching it and checking if it parses as valid RSS/Atom
pub async fn validate_feed(url: &str) -> Result<String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent("Yomi/1.0")
        .build()
        .context("Failed to create HTTP client")?;
    
    let response = tokio::time::timeout(
        Duration::from_secs(OPERATION_TIMEOUT_SECS),
        client.get(url).send()
    )
    .await
    .context("Request timed out")?
    .context("Failed to fetch feed")?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "HTTP error: {} - {}",
            response.status().as_u16(),
            response.status().canonical_reason().unwrap_or("Unknown error")
        ));
    }
    
    let content = response.bytes().await.context("Failed to read response body")?;
    
    // Try to parse the feed to validate it
    let feed = parser::parse(content.as_ref()).context("Failed to parse feed content")?;
    
    // Return the feed title if available, or a generic name
    Ok(feed.title
        .and_then(|t| Some(t.content))
        .unwrap_or_else(|| "Untitled Feed".to_string()))
}

/// Add a new feed to the configuration
pub async fn add_feed(config: &mut Config, url: &str, custom_name: Option<String>) -> Result<()> {
    // First validate the feed to make sure it's a valid RSS/Atom feed
    let feed_name = match custom_name {
        Some(name) => name,
        None => validate_feed(url).await?,
    };
    
    // Check if the feed URL already exists
    if config.feeds.values().any(|existing_url| existing_url == url) {
        return Err(anyhow::anyhow!("Feed with this URL already exists"));
    }
    
    // Add the feed
    config.feeds.insert(feed_name, url.to_string());
    
    Ok(())
}

/// Remove a feed from the configuration
pub fn remove_feed(config: &mut Config, target: &str) -> Result<()> {
    // Try to parse target as an index first
    if let Ok(index) = target.parse::<usize>() {
        if index >= config.feeds.len() {
            return Err(anyhow::anyhow!("Invalid feed index: {}", index));
        }
        
        // Get the key at this index
        let key = config.feeds.keys().nth(index)
            .ok_or_else(|| anyhow::anyhow!("Invalid feed index: {}", index))?
            .clone();
        
        config.feeds.remove(&key);
        return Ok(());
    }
    
    // If not an index, try to find by name
    if config.feeds.contains_key(target) {
        config.feeds.remove(target);
        Ok(())
    } else {
        Err(anyhow::anyhow!("Feed not found: {}", target))
    }
}

/// List all configured feeds
pub fn list_feeds(config: &Config) -> Result<()> {
    if config.feeds.is_empty() {
        println!("No feeds configured. Add feeds with 'yomi add <url>'");
        return Ok(());
    }
    
    println!("{:<3} {:<30} {}", "ID", "Feed Name", "URL");
    println!("{:-<3} {:-<30} {:-<40}", "", "", "");
    
    for (i, (name, url)) in config.feeds.iter().enumerate() {
        let truncated_name = if name.len() > 28 {
            format!("{}...", &name[..25])
        } else {
            name.clone()
        };
        
        println!("{:<3} {:<30} {}", i, truncated_name, url);
    }
    
    Ok(())
}

/// Refresh a specific feed or all feeds
pub async fn refresh_feeds(config: &Config, feed_name: Option<&str>) -> Result<()> {
    // Create a filtered map of feeds to refresh
    let feeds_to_refresh: HashMap<String, String> = match feed_name {
        Some(name) => {
            // Find a specific feed by name
            if let Some(url) = config.feeds.get(name) {
                let mut map = HashMap::new();
                map.insert(name.to_string(), url.clone());
                map
            } else {
                // Try to find by index
                if let Ok(index) = name.parse::<usize>() {
                    if let Some((name, url)) = config.feeds.iter().nth(index) {
                        let mut map = HashMap::new();
                        map.insert(name.clone(), url.clone());
                        map
                    } else {
                        return Err(anyhow::anyhow!("Feed not found at index: {}", index));
                    }
                } else {
                    return Err(anyhow::anyhow!("Feed not found: {}", name));
                }
            }
        },
        None => {
            // Refresh all feeds
            config.feeds.clone()
        }
    };
    
    if feeds_to_refresh.is_empty() {
        println!("No feeds to refresh.");
        return Ok(());
    }
    
    println!("Refreshing {} feed(s)...", feeds_to_refresh.len());
    
    for (name, url) in feeds_to_refresh.iter() {
        print!("Fetching '{}'... ", name);
        
        match fetch_feed(url).await {
            Ok(articles) => {
                println!("OK - {} articles found", articles.len());
                
                // Print first 5 articles
                if !articles.is_empty() {
                    println!("\nLatest articles:");
                    for (i, article) in articles.iter().take(5).enumerate() {
                        println!("  {}. {}", i+1, article.title);
                    }
                    
                    if articles.len() > 5 {
                        println!("  ... and {} more", articles.len() - 5);
                    }
                    println!();
                }
            },
            Err(e) => {
                println!("ERROR - {}", e);
            }
        }
    }
    
    Ok(())
}

// Simplified article for display in CLI mode
#[derive(Debug)]
struct Article {
    title: String,
}

/// Fetch a feed for CLI display
async fn fetch_feed(url: &str) -> Result<Vec<Article>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent("Yomi/1.0")
        .build()
        .context("Failed to create HTTP client")?;
    
    let response = tokio::time::timeout(
        Duration::from_secs(OPERATION_TIMEOUT_SECS),
        client.get(url).send()
    )
    .await
    .context("Request timed out")?
    .context("Failed to fetch feed")?;
    
    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "HTTP error: {} - {}",
            response.status().as_u16(),
            response.status().canonical_reason().unwrap_or("Unknown error")
        ));
    }
    
    let content = response.bytes().await.context("Failed to read response body")?;
    let feed = parser::parse(content.as_ref()).context("Failed to parse feed")?;
    
    let articles = feed.entries
        .into_iter()
        .map(|entry| {
            Article {
                title: entry.title
                    .map(|t| t.content)
                    .unwrap_or_else(|| "No title".to_string()),
            }
        })
        .collect();
    
    Ok(articles)
}