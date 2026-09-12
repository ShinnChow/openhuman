//! Classifies raw provider error strings into user-facing copy: budget
//! exhaustion, non-retryable rate limits, and fallback-chain exhaustion.
//! Split across `web_errors_part_01..03.rs` and `include!`d here.

include!("web_errors_part_01.rs");
include!("web_errors_part_02.rs");
include!("web_errors_part_03.rs");
