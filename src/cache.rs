use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use crate::config::Config;
use crate::secrets;

/// Metadata stored alongside the cache for freshness checks.
struct CacheMeta {
    date: String,         // YYYY-MM-DD
    timestamp: i64,       // epoch seconds
    secrets_hash: String, // hash of secrets file
}

/// Check if the cache needs to be refreshed.
pub fn needs_refresh(config: &Config) -> Result<bool> {
    // Force refresh
    if config.force {
        return Ok(true);
    }

    // No cache or meta file
    if !Path::new(&config.cache_file).exists() || !Path::new(&config.meta_file).exists() {
        return Ok(true);
    }

    // Read meta
    let meta = read_meta(config)?;

    // Secrets file changed
    let current_hash = secrets::hash_secrets_file(&config.secrets_file)?;
    if meta.secrets_hash != current_hash {
        return Ok(true);
    }

    let now = Local::now();

    // First login of the day
    let today = now.format("%Y-%m-%d").to_string();
    if meta.date != today {
        return Ok(true);
    }

    // TTL expired
    let elapsed = now.timestamp() - meta.timestamp;
    if elapsed > config.ttl as i64 {
        return Ok(true);
    }

    Ok(false)
}

/// Write resolved secrets to the cache file and update meta.
pub fn write_cache(config: &Config, resolved: &[(String, String)]) -> Result<()> {
    // Write cache file (export format)
    let mut content = String::new();
    for (key, value) in resolved {
        content.push_str(&format!(
            "export {}='{}'\n",
            key,
            value.replace('\'', "'\\''")
        ));
    }

    fs::write(&config.cache_file, &content)
        .with_context(|| format!("failed to write cache: {}", config.cache_file))?;

    // Set permissions to 600
    let perms = fs::Permissions::from_mode(0o600);
    fs::set_permissions(&config.cache_file, perms)?;

    // Write meta
    let now = Local::now();
    let secrets_hash = secrets::hash_secrets_file(&config.secrets_file)?;
    let meta_content = format!(
        "{}\n{}\n{}\n",
        now.format("%Y-%m-%d"),
        now.timestamp(),
        secrets_hash
    );

    fs::write(&config.meta_file, &meta_content)
        .with_context(|| format!("failed to write meta: {}", config.meta_file))?;

    let perms = fs::Permissions::from_mode(0o600);
    fs::set_permissions(&config.meta_file, perms)?;

    Ok(())
}

/// Read resolved secrets from the cache file.
pub fn read_cache(config: &Config) -> Result<Vec<(String, String)>> {
    let content = fs::read_to_string(&config.cache_file)
        .with_context(|| format!("failed to read cache: {}", config.cache_file))?;

    let mut resolved = Vec::new();
    for line in content.lines() {
        // Parse: export KEY='value'
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("export ") {
            if let Some(eq_pos) = rest.find('=') {
                let key = rest[..eq_pos].to_string();
                let value = rest[eq_pos + 1..]
                    .trim_start_matches('\'')
                    .trim_end_matches('\'')
                    .to_string();
                resolved.push((key, value));
            }
        }
    }

    Ok(resolved)
}

/// Get a human-readable cache age string.
pub fn cache_age(config: &Config) -> Result<String> {
    let meta = read_meta(config)?;
    let now = Local::now().timestamp();
    let diff = now - meta.timestamp;

    let hours = diff / 3600;
    let mins = (diff % 3600) / 60;

    if hours > 0 {
        Ok(format!("{}h{}m old", hours, mins))
    } else {
        Ok(format!("{}m old", mins))
    }
}

/// Read the meta file.
fn read_meta(config: &Config) -> Result<CacheMeta> {
    let content = fs::read_to_string(&config.meta_file)
        .with_context(|| format!("failed to read meta: {}", config.meta_file))?;

    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 3 {
        anyhow::bail!("malformed meta file: expected 3 lines, got {}", lines.len());
    }

    Ok(CacheMeta {
        date: lines[0].to_string(),
        timestamp: lines[1]
            .parse()
            .with_context(|| "failed to parse timestamp from meta")?,
        secrets_hash: lines[2].to_string(),
    })
}
