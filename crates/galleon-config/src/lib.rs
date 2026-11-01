//! Configuration schema, secrets, and related types for ClawCrew.

// `to_string()` inside `record!` `format!` args is the deliberate pattern
// for crossing a Serialize→string boundary; clippy can't tell those from
// redundant calls, so the lint is silenced at the crate root.
#![allow(clippy::unwrap_used, clippy::expect_used)] // RATCHET(BL-091): 27 per 2026-10-29; jangan menambah.
#![allow(unsafe_code)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::to_string_in_format_args)]
#![allow(clippy::useless_format)]

pub mod alias_refs;
pub mod api_error;
pub mod audit;
pub mod autonomy;
pub mod browse;
pub mod classifier;
pub mod comment_writer;
pub mod config_migration;
pub mod cost;
pub mod domain_matcher;
pub mod dto;
pub mod env_overrides;
pub mod field_visibility;
pub mod helpers;
pub mod identity;
pub mod integrations;
pub mod migration;
pub mod multi_agent;
pub mod pairing;
pub mod paths;
pub mod peers;
pub mod platform;
pub mod policy;
pub mod presets;
pub mod provider_aliases;
pub mod providers;
pub mod quickstart;
pub mod scattered_types;
pub mod schema;
#[cfg(feature = "schema-export")]
pub mod schema_markdown;
pub mod secrets;
pub mod sections;
pub mod skill_bundles;
pub mod thinking;
pub mod traits;
pub mod typed_value;
pub mod validation_warnings;
#[cfg(feature = "webauthn")]
pub mod webauthn;
pub mod write_lock;

/// Shim module so `Configurable` derive macro's generated `crate::config::*` paths resolve.
/// The macro was written assuming it runs inside the root crate where `mod config` exists.
pub mod config {
    pub use crate::helpers::*;
    pub use crate::traits::*;
}

/// Shim module so `Configurable` derive macro's generated `crate::security::*` paths resolve.
pub mod security {
    pub use crate::audit;
    pub use crate::policy::SecurityPolicy;
    pub use crate::secrets::SecretStore;
    #[cfg(feature = "webauthn")]
    pub use crate::webauthn;
}
