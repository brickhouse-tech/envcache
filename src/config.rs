/// Configuration for envcache, resolved from CLI args, env vars, or defaults.
pub struct Config {
    pub secrets_file: String,
    pub cache_file: String,
    pub meta_file: String,
    pub ttl: u64,
    pub force: bool,
}

impl Config {
    pub fn default_secrets_file() -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.envcache.secrets", home)
    }

    pub fn default_cache_file() -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.envrc.cache", home)
    }

    pub fn default_meta_file() -> String {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.envrc.cache.meta", home)
    }
}
