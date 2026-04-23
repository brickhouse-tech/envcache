use anyhow::Result;
use clap::Parser;
use envcache::cache;
use envcache::config::Config;
use envcache::resolve;
use envcache::secrets;

/// Cache environment secrets from 1Password (and other secret managers) with TTL.
///
/// Resolves secrets once, caches the results, and reuses them until a refresh
/// is triggered by TTL expiry, first login of the day, secrets file change,
/// or the --force flag.
#[derive(Parser, Debug)]
#[command(name = "envcache", version, about)]
struct Cli {
    /// Force refresh, ignoring cache
    #[arg(short, long)]
    force: bool,

    /// Path to secrets definition file
    #[arg(short, long, env = "ENVCACHE_SECRETS_FILE")]
    secrets_file: Option<String>,

    /// Path to cache file
    #[arg(short, long, env = "ENVCACHE_CACHE_FILE")]
    cache_file: Option<String>,

    /// Path to meta file
    #[arg(short, long, env = "ENVCACHE_META_FILE")]
    meta_file: Option<String>,

    /// Cache TTL in seconds (default: 28800 = 8 hours)
    #[arg(short, long, env = "ENVCACHE_TTL", default_value = "28800")]
    ttl: u64,

    /// Output format for shell eval
    #[arg(long, default_value = "export")]
    format: OutputFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    /// export VAR=value (for bash/zsh eval)
    Export,
    /// VAR=value (for .env file format)
    Dotenv,
    /// JSON object
    Json,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config = Config {
        secrets_file: cli
            .secrets_file
            .unwrap_or_else(Config::default_secrets_file),
        cache_file: cli.cache_file.unwrap_or_else(Config::default_cache_file),
        meta_file: cli.meta_file.unwrap_or_else(Config::default_meta_file),
        ttl: cli.ttl,
        force: cli.force,
    };

    let secrets = secrets::parse_secrets_file(&config.secrets_file)?;

    let resolved = if cache::needs_refresh(&config)? {
        eprintln!("[envcache] resolving secrets from 1Password...");
        let resolved = resolve::resolve_secrets(&secrets)?;
        let errors = secrets.len() - resolved.len();
        cache::write_cache(&config, &resolved)?;
        eprintln!(
            "[envcache] resolved {} secrets ({} errors)",
            resolved.len(),
            errors
        );
        resolved
    } else {
        let age = cache::cache_age(&config)?;
        eprintln!("[envcache] using cache ({})", age);
        cache::read_cache(&config)?
    };

    // Output resolved secrets in the requested format
    match cli.format {
        OutputFormat::Export => {
            for (key, value) in &resolved {
                println!("export {}='{}'", key, value.replace('\'', "'\\''"));
            }
        }
        OutputFormat::Dotenv => {
            for (key, value) in &resolved {
                println!("{}={}", key, value);
            }
        }
        OutputFormat::Json => {
            let map: std::collections::HashMap<&str, &str> = resolved
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            println!("{}", serde_json::to_string_pretty(&map)?);
        }
    }

    Ok(())
}
