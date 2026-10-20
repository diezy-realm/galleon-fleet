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
pub use galleon_channel_core::login_events;
pub mod login_probe;
pub mod login_relink;
#[cfg(feature = "channel-telegram")]
pub use galleon_channel_core::model_picker_delivery;
pub mod orchestrator;
pub mod paced_channel;
pub mod util;

// Always-compiled channels and utilities (no feature gate)
pub mod cli;
pub mod link_enricher;
pub mod transcription;
pub mod tts;
pub mod voice;

// RF-B1: the AMQP channel now lives in its own feature-crate. Re-export it
// under the historical `amqp` path so `crate::amqp::AmqpChannel` and callers
// keep resolving.
#[cfg(feature = "channel-amqp")]
pub use galleon_channel_amqp as amqp;
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
// RF-B1: the Discord channel now lives in its own feature-crate. Re-export it
// under the historical `discord` path so `crate::discord::DiscordChannel` and callers
// keep resolving.
#[cfg(feature = "channel-discord")]
pub use galleon_channel_discord as discord;
#[cfg(feature = "channel-discord")]
pub use galleon_channel_discord::discord_slash_state;
// RF-B1: the Email channel now lives in its own feature-crate. Re-export it
// under the historical `email_channel` path so `crate::email_channel::EmailChannel`
// and callers keep resolving.
#[cfg(feature = "channel-email")]
pub use galleon_channel_email as email_channel;
// RF-B1: the Filesystem channel now lives in its own feature-crate. Re-export it
// under the historical `filesystem` path so `crate::filesystem::FilesystemChannel`
// and callers keep resolving.
#[cfg(feature = "channel-filesystem")]
pub use galleon_channel_filesystem as filesystem;
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
// RF-B1: the Lark channel now lives in its own feature-crate. Re-export it
// under the historical `lark` path so `crate::lark::LarkChannel` and callers
// keep resolving.
#[cfg(feature = "channel-lark")]
pub use galleon_channel_lark as lark;
// RF-B1: the LINE channel now lives in its own feature-crate. Re-export it
// under the historical `line` path so `crate::line::LineChannel` and callers
// keep resolving.
#[cfg(feature = "channel-line")]
pub use galleon_channel_line as line;
// RF-B1: the Linq channel now lives in its own feature-crate. Re-export it
// under the historical `linq` path so `crate::linq::LinqChannel` and callers
// keep resolving.
#[cfg(feature = "channel-linq")]
pub use galleon_channel_linq as linq;
// RF-B1: the Matrix channel now lives in its own feature-crate. Re-export it
// under the historical `matrix` path so `crate::matrix::MatrixChannel` and callers
// keep resolving.
#[cfg(feature = "channel-matrix")]
pub use galleon_channel_matrix as matrix;
// RF-B1: the Mattermost channel now lives in its own feature-crate. Re-export it
// under the historical `mattermost` path so `crate::mattermost::MattermostChannel`
// and callers keep resolving.
#[cfg(feature = "channel-mattermost")]
pub use galleon_channel_mattermost as mattermost;
// RF-B1: the Mochat channel now lives in its own feature-crate. Re-export it
// under the historical `mochat` path so `crate::mochat::MochatChannel` and callers
// keep resolving.
#[cfg(feature = "channel-mochat")]
pub use galleon_channel_mochat as mochat;
// RF-B1: the Nextcloud Talk channel now lives in its own feature-crate. Re-export it
// under the historical `nextcloud_talk` path so `crate::nextcloud_talk::NextcloudTalkChannel`
// and callers keep resolving.
#[cfg(feature = "channel-nextcloud")]
pub use galleon_channel_nextcloud as nextcloud_talk;
// RF-B1: the Nostr channel now lives in its own feature-crate. Re-export it
// under the historical `nostr` path so `crate::nostr::NostrChannel` and callers
// keep resolving.
#[cfg(feature = "channel-nostr")]
pub use galleon_channel_nostr as nostr;
// RF-B1: the Notion channel now lives in its own feature-crate. Re-export it
// under the historical `notion` path so `crate::notion::NotionChannel` and callers
// keep resolving.
#[cfg(feature = "channel-notion")]
pub use galleon_channel_notion as notion;
// RF-B1: the QQ channel now lives in its own feature-crate. Re-export it
// under the historical `qq` path so `crate::qq::QQChannel` and callers
// keep resolving.
#[cfg(feature = "channel-qq")]
pub use galleon_channel_qq as qq;
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
// RF-B1: the Telegram channel now lives in its own feature-crate. Re-export it
// under the historical `telegram` path so `crate::telegram::TelegramChannel` and callers
// keep resolving.
#[cfg(feature = "channel-telegram")]
pub use galleon_channel_telegram as telegram;
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
// RF-B1: the Webhook channel now lives in its own feature-crate. Re-export it
// under the historical `webhook` path so `crate::webhook::WebhookChannel` and
// callers keep resolving.
#[cfg(feature = "channel-webhook")]
pub use galleon_channel_webhook as webhook;
// RF-B1: the WeChat channel now lives in its own feature-crate. Re-export it
// under the historical `wechat` path so `crate::wechat::WeChatChannel` and callers
// keep resolving.
#[cfg(feature = "channel-wechat")]
pub use galleon_channel_wechat as wechat;
// RF-B1: the WeCom channel now lives in its own feature-crate. Re-export it
// under the historical `wecom` path so callers keep resolving.
#[cfg(feature = "channel-wecom")]
pub use galleon_channel_wecom as wecom;
// RF-B1: the WeCom-WS channel now lives in its own feature-crate. Re-export it
// under the historical `wecom_ws` path so callers keep resolving.
#[cfg(feature = "channel-wecom-ws")]
pub use galleon_channel_wecom_ws as wecom_ws;
#[cfg(any(feature = "channel-whatsapp-cloud", feature = "whatsapp-web"))]
pub mod whatsapp;
#[cfg(feature = "whatsapp-web")]
pub mod whatsapp_storage;
#[cfg(feature = "whatsapp-web")]
pub mod whatsapp_web;
