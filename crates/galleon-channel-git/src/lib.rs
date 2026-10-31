//! Git-forge channel — converse with the agent through a forge's issue and
//! pull-request comments, and surface repository events (PR lifecycle,
//! review comments, CI outcomes, releases) through a per-event routing
//! table.

#![allow(clippy::unwrap_used, clippy::expect_used)] // RATCHET(BL-091): 1 per 2026-10-29; jangan menambah.

pub mod channel;
pub mod events;
pub mod poll;
pub mod providers;
pub mod router;
pub mod traits;
pub mod types;

pub use channel::GitChannel;
pub use events::GitEvent;
pub use router::{RouteAction, TransportPlan};
pub use traits::{FetchPage, GitProvider, ReactionTarget, SelfIdentity};
pub use types::{ForgeMethod, ForgeRequest, ForgeResponse, GitChannelError, IssueRef, RepoRef};
