//! Built-in agent archetypes.
//!
//! Each submodule below is one shipped agent and owns the same three
//! files, so none of the 30 leaf `mod.rs` files need their own doc
//! comment:
//!
//! * `agent.toml`  — id, `when_to_use`, model, tool allowlist, sandbox,
//!   iteration cap, and `omit_*` flags. Parsed directly into
//!   [`crate::agent::harness::definition::AgentDefinition`].
//! * `prompt.md`   — legacy static prompt body, kept for reference and as
//!   the workspace-override target.
//! * `prompt.rs`   — exposes `pub fn build(&PromptContext) ->
//!   anyhow::Result<String>`, wired into `PromptSource::Dynamic` by
//!   [`loader::BUILTINS`] so the prompt can branch on runtime state
//!   (available tools, user profile, connected integrations, model hint).
//!
//! A handful of archetypes (currently only `researcher`) additionally own a
//! `graph.rs` exposing `fn graph() -> AgentGraph` for a bespoke turn graph;
//! see [`loader::BuiltinAgent::graph_fn`].
//!
//! See the [package README](../README.md) for what each archetype does and
//! [`loader`] for how this list turns into the running registry.

mod loader;

pub mod archivist;
pub mod code_executor;
pub mod context_scout;
pub mod critic;
pub mod crypto_agent;
#[cfg(feature = "flows")]
pub mod flow_memory_agent;
pub mod goals_agent;
pub mod help;
pub mod image_agent;
pub mod integrations_agent;
#[cfg(feature = "mcp")]
pub mod mcp_agent;
pub mod mcp_setup;
pub mod morning_briefing;
pub mod orchestrator;
pub mod planner;
pub mod presentation_agent;
pub mod profile_memory_agent;
pub mod researcher;
pub mod scheduler_agent;
pub mod settings_agent;
pub mod skill_creator;
pub mod summarizer;
pub mod task_manager_agent;
pub mod tool_maker;
pub mod tools_agent;
pub mod trigger_reactor;
pub mod trigger_triage;
pub mod video_agent;
pub mod vision_agent;

pub use loader::{load_builtins, validate_tier_hierarchy, BuiltinAgent, BUILTINS};
