//! Channel implementations and orchestration for messaging platform integrations.

#![allow(
    clippy::to_string_in_format_args,
    clippy::useless_format,
    clippy::explicit_auto_deref
)]
#![cfg_attr(feature = "channel-matrix", recursion_limit = "256")]

pub mod allowlist;
// Ungated: the reader-aligned paired-identity writer is now shared by every
// writer of an `external_peers` grant, including the CLI/API bind core and the
// Telegram and LINE pairing paths, none of which are wechat/whatsapp-web.
pub(crate) mod identity_persist;
pub mod listing;
pub mod login_events;
pub mod login_probe;
pub mod login_relink;
#[cfg(feature = "channel-telegram")]
pub(crate) mod model_picker_delivery;
pub mod orchestrator;
pub mod paced_channel;
pub mod util;

// Always-compiled channels and utilities (no feature gate)
pub mod cli;
pub mod link_enricher;
pub mod transcription;
pub mod tts;
pub mod voice;

// Feature-gated channels
#[cfg(feature = "channel-amqp")]
pub mod amqp;
// RF-B1: the Bluesky channel now lives in its own feature-crate. Re-export it
// under the historical `bluesky` path so `crate::bluesky::BlueskyChannel` and
// the orchestrator's `pub use crate::bluesky::BlueskyChannel` keep resolving.
#[cfg(feature = "channel-bluesky")]
pub use galleon_channel_bluesky as bluesky;
// RF-B1: the ClawdTalk channel now lives in its own feature-crate. Re-export it
// under the historical `clawdtalk` path so `crate::clawdtalk::ClawdTalkChannel` and callers
// keep resolving.
#[cfg(feature = "channel-clawdtalk")]
pub use galleon_channel_clawdtalk as clawdtalk;
// RF-B1: the DingTalk channel now lives in its own feature-crate. Re-export it
// under the historical `dingtalk` path so `crate::dingtalk::DingTalkChannel` and callers
// keep resolving.
#[cfg(feature = "channel-dingtalk")]
pub use galleon_channel_dingtalk as dingtalk;
#[cfg(feature = "channel-discord")]
pub mod discord;
#[cfg(feature = "channel-discord")]
pub mod discord_slash_state;
#[cfg(feature = "channel-email")]
pub mod email_channel;
#[cfg(feature = "channel-filesystem")]
pub mod filesystem;
#[cfg(feature = "channel-git")]
pub mod git;
#[cfg(feature = "channel-email")]
pub mod gmail_push;
#[cfg(feature = "channel-imessage")]
pub use galleon_channel_imessage as imessage;
// RF-B1: the IRC channel now lives in its own feature-crate. Re-export it
// under the historical `irc` path so `crate::irc::IrcChannel` and the
// orchestrator's `pub use crate::irc::IrcChannel` keep resolving.
#[cfg(feature = "channel-irc")]
pub use galleon_channel_irc as irc;
#[cfg(feature = "channel-lark")]
pub mod lark;
#[cfg(feature = "channel-line")]
pub mod line;
#[cfg(feature = "channel-linq")]
pub mod linq;
#[cfg(feature = "channel-matrix")]
pub mod matrix;
#[cfg(feature = "channel-mattermost")]
pub mod mattermost;
// RF-B1: the Mochat channel now lives in its own feature-crate. Re-export it
// under the historical `mochat` path so `crate::mochat::MochatChannel` and callers
// keep resolving.
#[cfg(feature = "channel-mochat")]
pub use galleon_channel_mochat as mochat;
#[cfg(feature = "channel-nextcloud")]
pub mod nextcloud_talk;
// RF-B1: the Nostr channel now lives in its own feature-crate. Re-export it
// under the historical `nostr` path so `crate::nostr::NostrChannel` and callers
// keep resolving.
#[cfg(feature = "channel-nostr")]
pub use galleon_channel_nostr as nostr;
#[cfg(feature = "channel-notion")]
pub mod notion;
#[cfg(feature = "channel-qq")]
pub mod qq;
// RF-B1: the Reddit channel now lives in its own feature-crate. Re-export it
// under the historical `reddit` path so `crate::reddit::RedditChannel` and callers
// keep resolving.
#[cfg(feature = "channel-reddit")]
pub use galleon_channel_reddit as reddit;
// RF-B1: the Signal channel now lives in its own feature-crate. Re-export it
// under the historical `signal` path so `crate::signal::SignalChannel` and the
// orchestrator's `pub use crate::signal::SignalChannel` keep resolving.
#[cfg(feature = "channel-signal")]
pub use galleon_channel_signal as signal;
#[cfg(feature = "channel-slack")]
pub mod slack;
#[cfg(feature = "channel-telegram")]
pub mod telegram;
// RF-B1: the Twitch channel now lives in its own feature-crate. Re-export it
// under the historical `twitch` path so `crate::twitch::TwitchChannel` and callers
// keep resolving.
#[cfg(feature = "channel-twitch")]
pub use galleon_channel_twitch as twitch;
// RF-B1: the Twitter channel now lives in its own feature-crate. Re-export it
// under the historical `twitter` path so `crate::twitter::TwitterChannel` and callers
// keep resolving.
#[cfg(feature = "channel-twitter")]
pub use galleon_channel_twitter as twitter;
#[cfg(feature = "channel-voice-call")]
pub mod voice_call;
#[cfg(feature = "voice-wake")]
pub mod voice_wake;
#[cfg(feature = "channel-webhook")]
pub mod webhook;
#[cfg(feature = "channel-wechat")]
pub mod wechat;
#[cfg(feature = "channel-wecom")]
pub mod wecom;
#[cfg(feature = "channel-wecom-ws")]
pub mod wecom_ws;
#[cfg(any(feature = "channel-whatsapp-cloud", feature = "whatsapp-web"))]
pub mod whatsapp;
#[cfg(feature = "whatsapp-web")]
pub mod whatsapp_storage;
#[cfg(feature = "whatsapp-web")]
pub mod whatsapp_web;
