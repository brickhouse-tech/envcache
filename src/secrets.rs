use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// A secret definition parsed from the secrets file.
#[derive(Debug, Clone)]
pub struct SecretDef {
    pub var_name: String,
    pub op_uri: String,
    pub post_processor: Option<PostProcessor>,
}

#[derive(Debug, Clone)]
pub enum PostProcessor {
    StripWhitespace,
}

/// Parse the secrets file into a list of secret definitions.
///
/// Format: VAR_NAME|op://vault/item/field[|strip_whitespace]
/// Lines starting with # are comments. Blank lines are ignored.
pub fn parse_secrets_file(path: &str) -> Result<Vec<SecretDef>> {
    let path = Path::new(path);
    if !path.exists() {
        anyhow::bail!(
            "secrets file not found: {}\nCreate it with lines like: MY_VAR|op://vault/item/field",
            path.display()
        );
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read secrets file: {}", path.display()))?;

    let mut secrets = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and blank lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = line.splitn(3, '|').collect();
        if parts.len() < 2 {
            eprintln!("[envcache] WARNING: skipping malformed line: {}", line);
            continue;
        }

        let var_name = parts[0].to_string();
        let op_uri = parts[1].to_string();
        let post_processor = parts.get(2).and_then(|p| match *p {
            "strip_whitespace" => Some(PostProcessor::StripWhitespace),
            other => {
                eprintln!(
                    "[envcache] WARNING: unknown post-processor '{}' for {}",
                    other, var_name
                );
                None
            }
        });

        secrets.push(SecretDef {
            var_name,
            op_uri,
            post_processor,
        });
    }

    Ok(secrets)
}

/// Compute a hash of the secrets file for change detection.
pub fn hash_secrets_file(path: &str) -> Result<String> {
    let content = fs::read(path)
        .with_context(|| format!("failed to read secrets file for hashing: {}", path))?;

    // Simple hash: use the content length + first/last bytes + a rolling sum.
    // Not cryptographic, just for change detection (like md5 in the shell version).
    let mut hash: u64 = content.len() as u64;
    for (i, byte) in content.iter().enumerate() {
        hash = hash.wrapping_add((*byte as u64).wrapping_mul(i as u64 + 1));
    }
    Ok(format!("{:016x}", hash))
}
