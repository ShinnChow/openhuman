//! `start_chat`/`cancel_*`/`channel_web_*` operations: session cache
//! (`THREAD_SESSIONS`), in-flight/parallel turn tracking, and per-thread
//! budget-exhaustion correlation. Split across `ops_part_01..03.rs` and
//! `include!`d here rather than declared as submodules.

#[cfg(test)]
#[path = "ops_budget_correlation_tests_tests.rs"]
mod budget_correlation_tests;
include!("ops_part_01.rs");
include!("ops_part_02.rs");
include!("ops_part_03.rs");
