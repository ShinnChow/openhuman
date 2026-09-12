//! RPC schemas and controller registration for conversation threads.

#[cfg(test)]
#[path = "schemas_tests.rs"]
mod tests;

mod handlers;
mod registry;
mod schema_defs;

pub use registry::{all_controller_schemas, all_registered_controllers};

#[cfg(test)]
pub(crate) use schema_defs::schemas;
