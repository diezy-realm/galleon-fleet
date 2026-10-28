//! Invariant R-04: Crate dependency direction gate.
//!
//! Enforces the layered dependency direction rules defined in
//! standard-docs/tech-standart/code-quality/rust-module-size-and-decomposition-standard.md §5:
//! - galleon-api is a leaf: must not depend on higher-level workspace crates (runtime, gateway, channels, tools, providers).
//! - galleon-plugins must not depend on galleon-runtime.
//! - galleon-channel-* must not depend on galleon-runtime directly.

use std::{fs, path::Path};

fn has_active_dependency(content: &str, dep_name: &str) -> bool {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed.starts_with(dep_name) {
            let rest = &trimmed[dep_name.len()..];
            if rest.starts_with('.') || rest.starts_with(' ') || rest.starts_with('=') {
                return true;
            }
        }
    }
    false
}

#[test]
fn crate_dependency_direction_strictly_enforced() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // 1. Check galleon-api is a leaf
    let api_cargo = root.join("crates/galleon-api/Cargo.toml");
    if api_cargo.exists() {
        let content = fs::read_to_string(&api_cargo).expect("galleon-api/Cargo.toml readable");
        for forbidden in [
            "galleon-runtime",
            "galleon-gateway",
            "galleon-channels",
            "galleon-tools",
            "galleon-providers",
        ] {
            assert!(
                !has_active_dependency(&content, forbidden),
                "galleon-api is a leaf crate and must not depend on {forbidden}"
            );
        }
    }

    // 2. Check galleon-plugins does not depend on galleon-runtime
    let plugins_cargo = root.join("crates/galleon-plugins/Cargo.toml");
    if plugins_cargo.exists() {
        let content =
            fs::read_to_string(&plugins_cargo).expect("galleon-plugins/Cargo.toml readable");
        assert!(
            !has_active_dependency(&content, "galleon-runtime"),
            "galleon-plugins must not depend on galleon-runtime"
        );
    }

    // 3. Check channel crates do not depend on galleon-runtime
    for channel_crate in [
        "crates/galleon-channels",
        "crates/galleon-channel-core",
        "crates/galleon-channel-signal",
        "crates/galleon-channel-bluesky",
        "crates/galleon-channel-imessage",
        "crates/galleon-channel-irc",
        "crates/galleon-channel-nostr",
        "crates/galleon-channel-twitch",
        "crates/galleon-channel-dingtalk",
        "crates/galleon-channel-mochat",
        "crates/galleon-channel-clawdtalk",
        "crates/galleon-channel-reddit",
        "crates/galleon-channel-twitter",
        "crates/galleon-channel-notion",
        "crates/galleon-channel-linq",
        "crates/galleon-channel-nextcloud",
        "crates/galleon-channel-filesystem",
        "crates/galleon-channel-amqp",
        "crates/galleon-channel-wecom",
        "crates/galleon-channel-webhook",
        "crates/galleon-channel-email",
        "crates/galleon-channel-qq",
        "crates/galleon-channel-line",
        "crates/galleon-channel-mattermost",
        "crates/galleon-channel-lark",
        "crates/galleon-channel-wechat",
        "crates/galleon-channel-telegram",
        "crates/galleon-channel-wecom-ws",
        "crates/galleon-channel-discord",
        "crates/galleon-channel-matrix",
        "crates/galleon-channel-slack",
        "crates/galleon-channel-git",
        "crates/galleon-channel-voice-call",
        "crates/galleon-channel-whatsapp",
    ] {
        let cargo_path = root.join(channel_crate).join("Cargo.toml");
        if cargo_path.exists() {
            let content = fs::read_to_string(&cargo_path)
                .unwrap_or_else(|_| panic!("Failed to read {channel_crate}/Cargo.toml"));
            assert!(
                !has_active_dependency(&content, "galleon-runtime"),
                "Channel crate {channel_crate} must not depend on galleon-runtime"
            );
        }
    }
}
