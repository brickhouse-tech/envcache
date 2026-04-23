use envcache::cache::{needs_refresh, read_cache, write_cache};
use envcache::config::Config;
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

fn setup_test_config(dir: &TempDir, secrets_content: &str) -> Config {
    let secrets_path = dir.path().join(".envcache.secrets");
    let mut f = fs::File::create(&secrets_path).unwrap();
    f.write_all(secrets_content.as_bytes()).unwrap();

    Config {
        secrets_file: secrets_path.to_str().unwrap().to_string(),
        cache_file: dir
            .path()
            .join(".envrc.cache")
            .to_str()
            .unwrap()
            .to_string(),
        meta_file: dir
            .path()
            .join(".envrc.cache.meta")
            .to_str()
            .unwrap()
            .to_string(),
        ttl: 28800,
        force: false,
    }
}

#[test]
fn test_needs_refresh_no_cache() {
    let dir = TempDir::new().unwrap();
    let config = setup_test_config(&dir, "VAR|op://vault/item/field\n");
    assert!(needs_refresh(&config).unwrap());
}

#[test]
fn test_needs_refresh_force() {
    let dir = TempDir::new().unwrap();
    let mut config = setup_test_config(&dir, "VAR|op://vault/item/field\n");
    config.force = true;
    assert!(needs_refresh(&config).unwrap());
}

#[test]
fn test_write_and_read_cache() {
    let dir = TempDir::new().unwrap();
    let config = setup_test_config(&dir, "VAR|op://vault/item/field\n");

    let secrets = vec![
        ("MY_KEY".to_string(), "secret-value".to_string()),
        ("OTHER".to_string(), "other-value".to_string()),
    ];

    write_cache(&config, &secrets).unwrap();
    let read_back = read_cache(&config).unwrap();

    assert_eq!(read_back.len(), 2);
    assert_eq!(
        read_back[0],
        ("MY_KEY".to_string(), "secret-value".to_string())
    );
    assert_eq!(
        read_back[1],
        ("OTHER".to_string(), "other-value".to_string())
    );
}

#[test]
fn test_cache_permissions() {
    let dir = TempDir::new().unwrap();
    let config = setup_test_config(&dir, "VAR|op://vault/item/field\n");

    let secrets = vec![("KEY".to_string(), "val".to_string())];
    write_cache(&config, &secrets).unwrap();

    let metadata = fs::metadata(&config.cache_file).unwrap();
    let mode = metadata.permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn test_needs_refresh_after_write() {
    let dir = TempDir::new().unwrap();
    let config = setup_test_config(&dir, "VAR|op://vault/item/field\n");

    let secrets = vec![("KEY".to_string(), "val".to_string())];
    write_cache(&config, &secrets).unwrap();

    // Should not need refresh — cache is fresh
    assert!(!needs_refresh(&config).unwrap());
}

#[test]
fn test_needs_refresh_secrets_changed() {
    let dir = TempDir::new().unwrap();
    let config = setup_test_config(&dir, "VAR|op://vault/item/field\n");

    let secrets = vec![("KEY".to_string(), "val".to_string())];
    write_cache(&config, &secrets).unwrap();

    // Modify secrets file
    fs::write(
        &config.secrets_file,
        "VAR|op://vault/item/field\nNEW|op://vault/item/new\n",
    )
    .unwrap();

    assert!(needs_refresh(&config).unwrap());
}
