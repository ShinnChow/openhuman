use super::*;
use serde_json::json;

/// Assembled rather than written out so a repository secret scanner does
/// not read the fixture as a real key block.
fn private_key_fixture(kind: &str, body: &str) -> String {
    format!("-----BEGIN {kind}-----\n{body}\n-----END {kind}-----")
}

fn redacts(input: &str, token: &str) {
    let out = redact_pii(input);
    assert!(
        out.value.contains(token),
        "expected {token} in output. input={input:?} output={out:?}"
    );
}

fn unchanged(input: &str) {
    let out = redact_pii(input);
    assert_eq!(
        out.value, input,
        "expected no change; report={:?}",
        out.report
    );
    assert_eq!(out.report.pii_redactions, 0);
}

#[path = "sanitize_and_pii_id_tests.rs"]
mod sanitize_and_pii_id_tests;
#[path = "prefilter_and_checksum_tests.rs"]
mod prefilter_and_checksum_tests;
