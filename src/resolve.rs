use anyhow::Result;
use std::process::Command;

use crate::secrets::{PostProcessor, SecretDef};

/// Resolve all secrets by calling `op read` for each definition.
/// Returns a Vec of (var_name, resolved_value) pairs.
pub fn resolve_secrets(secrets: &[SecretDef]) -> Result<Vec<(String, String)>> {
    let mut resolved = Vec::new();

    for secret in secrets {
        match resolve_one(&secret.op_uri) {
            Ok(mut value) => {
                // Apply post-processing
                if let Some(ref processor) = secret.post_processor {
                    value = apply_post_processor(&value, processor);
                }
                resolved.push((secret.var_name.clone(), value));
            }
            Err(e) => {
                eprintln!(
                    "[envcache] WARNING: failed to resolve {}: {}",
                    secret.var_name, e
                );
            }
        }
    }

    Ok(resolved)
}

/// Resolve a single secret via `op read`.
fn resolve_one(op_uri: &str) -> Result<String> {
    let output = Command::new("op").args(["read", op_uri]).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("op read failed: {}", stderr.trim());
    }

    let value = String::from_utf8(output.stdout)?;
    Ok(value.trim_end_matches('\n').to_string())
}

/// Apply a post-processor to a resolved value.
pub fn apply_post_processor(value: &str, processor: &PostProcessor) -> String {
    match processor {
        PostProcessor::StripWhitespace => value.chars().filter(|c| !c.is_whitespace()).collect(),
    }
}
