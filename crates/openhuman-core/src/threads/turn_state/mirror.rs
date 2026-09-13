//! Translate [`AgentProgress`] events into [`TurnState`] mutations and
//! flush snapshots to disk at iteration / tool boundaries.
//!
//! Used by the web-channel progress bridge to keep an authoritative,
//! restart-survivable record of the in-flight turn alongside the live
//! socket emissions. High-frequency deltas (text, thinking, tool args)
//! mutate the in-memory snapshot but do not trigger a disk flush —
//! anything more granular than an iteration / tool boundary would
//! thrash the filesystem under streaming load.
//!
//! On terminal completion the snapshot file is deleted. If the bridge
//! exits without ever observing [`AgentProgress::TurnCompleted`] (for
//! example because the agent loop returned an error), the snapshot is
//! flagged [`TurnLifecycle::Interrupted`] and persisted so the UI can
//! surface a retry affordance.

#[cfg(test)]
#[path = "mirror_tests.rs"]
mod tests;

mod caps;
mod lifecycle;
mod observe;
mod state;

pub use state::TurnStateMirror;

#[cfg(test)]
use super::store::TurnStateStore;
#[cfg(test)]
use super::types::{
    SubagentToolCall, SubagentTranscriptItem, ToolTimelineStatus, TranscriptItem, TurnLifecycle,
    TurnPhase,
};
#[cfg(test)]
pub(crate) use caps::MAX_PERSISTED_TRANSCRIPT_ITEM;
