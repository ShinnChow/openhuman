//! Agent-owned dialogue and control tools.
//!
//! Unlike domain tools (files, memory, search, ...), the tools declared here
//! act on the agent loop itself rather than on external state:
//!
//! - [`ask_clarification::AskClarificationTool`] — early-exit a turn to ask
//!   the user a clarifying question instead of guessing.
//! - [`delegate::DelegateTool`] and
//!   [`delegate_to_personality::DelegateToPersonalityTool`] — hand a subtask
//!   off to a differently configured sub-agent or a named personality.
//! - [`plan_exit::PlanExitTool`] — mark a plan-mode pass complete with the
//!   [`plan_exit::PLAN_EXIT_MARKER`] the harness greps for on a plan→build
//!   hand-off.
//! - [`remember_preference`] / [`save_preference`] — capture user
//!   preferences during a turn.
//! - `run_workflow` — spawn and await a `skill_runtime` workflow run; gated
//!   behind the `skills` feature so the tool is omitted from the catalog
//!   entirely (not degraded to a disabled-error) on builds without it.
//! - [`todo::TodoTool`] and [`update_task::UpdateTaskTool`] — maintain the
//!   agent's todo/task board.
//!
//! All nine tools are re-exported through `crate::tools`
//! (`pub use crate::agent::tools::*;` in `tools/mod.rs`), so callers should
//! reach them via `crate::tools` rather than this module directly.
mod ask_clarification;
mod delegate;
mod delegate_to_personality;
mod plan_exit;
pub mod remember_preference;
// Pure `skill_runtime` client (spawn + await a workflow run) — compiled out
// with the `skills` gate so the tool list OMITS these rather than degrading
// them to a disabled-error.
#[cfg(feature = "skills")]
mod run_workflow;
pub mod save_preference;
mod todo;
mod update_task;

pub use ask_clarification::AskClarificationTool;
pub use delegate::DelegateTool;
pub use delegate_to_personality::DelegateToPersonalityTool;
pub use plan_exit::{PlanExitTool, PLAN_EXIT_MARKER};
pub use remember_preference::RememberPreferenceTool;
#[cfg(feature = "skills")]
pub use run_workflow::{
    AwaitWorkflowTool, RunWorkflowTool, AWAIT_WORKFLOW_TOOL_NAME, RUN_WORKFLOW_TOOL_NAME,
};
pub use save_preference::SavePreferenceTool;
pub use todo::TodoTool;
pub use update_task::UpdateTaskTool;
