//! RPC operations for conversation thread management.

#[cfg(test)]
#[path = "ops_tests.rs"]
mod tests;

mod crud;
mod purge;
mod support;
mod title_generation;
mod transcript;
mod turn_state_ops;
mod usage;

pub use crud::{
    message_append, message_update, messages_list, thread_create_new, thread_delete,
    thread_update_labels, thread_update_title, thread_upsert, threads_list, transcript_search,
};
pub use purge::threads_purge;
pub use title_generation::thread_generate_title;
pub use transcript::{transcript_get, TranscriptGetRequest};
pub use turn_state_ops::{
    turn_state_clear, turn_state_get, turn_state_get_turn, turn_state_history, turn_state_list,
};
pub use usage::{token_usage, SubagentUsageDto, ThreadTokenUsageRequest, ThreadTokenUsageResponse};
