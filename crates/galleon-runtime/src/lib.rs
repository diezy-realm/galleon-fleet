#![allow(clippy::unwrap_used, clippy::expect_used)] // RATCHET(BL-091): 219 per 2026-10-29; jangan menambah.
#![allow(unsafe_code)]
#![allow(
    clippy::to_string_in_format_args,
    clippy::useless_format,
    clippy::manual_inspect,
    clippy::collapsible_if,
    clippy::disallowed_macros,
    clippy::disallowed_methods,
    clippy::derivable_impls,
    clippy::field_reassign_with_default
)]
//! Agent runtime — orchestration, security, observability, cron, SOP, skills, hardware, and more.

pub mod cli_input;
pub mod identity;
pub mod migration;
pub mod util;

pub mod acp;
pub mod agent;
pub mod approval;
pub mod browse;
pub mod calendar;
pub mod control_plane;
pub mod cost;
pub mod cron;
pub mod daemon;
pub mod doctor;
pub mod enroll;
pub mod health;
pub mod heartbeat;
pub mod hooks;
// RF-B: the i18n body now lives in the standalone `galleon-i18n` crate (so
// channels/gateway can localize without depending on this 288k-LOC runtime).
// Re-export it under the historical `i18n` path so every existing
// `galleon_runtime::i18n::*` caller keeps resolving unchanged.
pub use galleon_i18n as i18n;
pub mod integrations;
pub mod observability;
pub mod peers;
pub mod platform;
pub mod plugin_runtime;
pub mod process_stats;
pub mod quickstart;
pub mod rag;
pub mod relay;
pub mod restart;
pub mod routines;
pub mod security;
pub mod service;
pub mod session;
pub mod skills;
pub mod sop;
pub use sop::{RuntimeSopRunner, build_sop_engine};
pub mod subagent;
pub use agent::{Agent, RuntimeAgentRunner, install_agent_runner, process_message};
pub mod tools;
pub mod trust;
pub mod tui_identity;
pub mod tunnel;
