//! ModelProvider subsystem — re-exported from `galleon-providers`.

pub use galleon_providers::*;

// Keep traits.rs as a file module so its #[cfg(test)] block compiles.
#[path = "traits.rs"]
pub mod traits;
