use envcache::secrets::{hash_secrets_file, parse_secrets_file, PostProcessor};
use std::io::Write;
use tempfile::NamedTempFile;

fn write_temp_secrets(content: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(content.as_bytes()).unwrap();
    f
}

#[test]
fn test_parse_basic() {
    let f = write_temp_secrets("MY_VAR|op://vault/item/password\n");
    let secrets = parse_secrets_file(f.path().to_str().unwrap()).unwrap();
    assert_eq!(secrets.len(), 1);
    assert_eq!(secrets[0].var_name, "MY_VAR");
    assert_eq!(secrets[0].op_uri, "op://vault/item/password");
    assert!(secrets[0].post_processor.is_none());
}

#[test]
fn test_parse_with_post_processor() {
    let f = write_temp_secrets("TOKEN|op://vault/item/password|strip_whitespace\n");
    let secrets = parse_secrets_file(f.path().to_str().unwrap()).unwrap();
    assert_eq!(secrets.len(), 1);
    assert!(matches!(
        secrets[0].post_processor,
        Some(PostProcessor::StripWhitespace)
    ));
}

#[test]
fn test_parse_skips_comments_and_blanks() {
    let f = write_temp_secrets("# comment\n\nMY_VAR|op://vault/item/field\n# another\n");
    let secrets = parse_secrets_file(f.path().to_str().unwrap()).unwrap();
    assert_eq!(secrets.len(), 1);
}

#[test]
fn test_parse_missing_file() {
    let result = parse_secrets_file("/tmp/nonexistent-envcache-test");
    assert!(result.is_err());
}

#[test]
fn test_hash_changes_on_content_change() {
    let f1 = write_temp_secrets("VAR1|op://vault/item/field\n");
    let f2 = write_temp_secrets("VAR1|op://vault/item/field\nVAR2|op://vault/item/other\n");
    let h1 = hash_secrets_file(f1.path().to_str().unwrap()).unwrap();
    let h2 = hash_secrets_file(f2.path().to_str().unwrap()).unwrap();
    assert_ne!(h1, h2);
}
