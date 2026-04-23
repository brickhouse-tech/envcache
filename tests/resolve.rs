use envcache::resolve::apply_post_processor;
use envcache::secrets::PostProcessor;

#[test]
fn test_strip_whitespace() {
    let result = apply_post_processor("hello \n world\r\n", &PostProcessor::StripWhitespace);
    assert_eq!(result, "helloworld");
}

#[test]
fn test_strip_whitespace_no_change() {
    let result = apply_post_processor("clean-value", &PostProcessor::StripWhitespace);
    assert_eq!(result, "clean-value");
}
