//! Shared paired-identity persistence for QR-pairing and pairing channels.
//!
//! When a pairing completes, the linked account identity becomes an
//! authorized external peer. The canonical home for that authorization is
//! a `[peer_groups.<name>]` entry in `config.toml` whose `channel` field
//! is the dotted `<channel_type>.<alias>` instance ref.

use galleon_config::schema::Config;

/// The conflict message when a matching `ignore` already denies `identity`,
/// or `None` when a pairing write would take effect.
pub fn external_peer_deny_conflict(
    cfg: &Config,
    channel_type: &str,
    alias: &str,
    identities: &[&str],
    match_fn: impl Fn(&str, &str) -> bool,
) -> Option<String> {
    let usable: Vec<&str> = identities
        .iter()
        .map(|identity| identity.trim())
        .filter(|identity| !identity.is_empty())
        .collect();
    if usable.is_empty() {
        return None;
    }
    let resolved = cfg.channel_external_peers(channel_type, alias);
    crate::allowlist::pairing_deny_conflict(&resolved, channel_type, alias, &usable, match_fn)
}

/// The `peer_groups` key whose `channel` is exactly `<channel_type>.<alias>`,
/// which is where a write for that instance lands.
#[must_use]
pub fn instance_group_key(cfg: &Config, channel_type: &str, alias: &str) -> Option<String> {
    let dotted_ref = format!("{channel_type}.{alias}");
    let conventional_key = format!("{channel_type}_{alias}");
    if cfg
        .peer_groups
        .get(&conventional_key)
        .is_some_and(|group| group.channel.as_str() == dotted_ref)
    {
        return Some(conventional_key);
    }
    cfg.peer_groups
        .iter()
        .filter(|(_, group)| group.channel.as_str() == dotted_ref)
        .map(|(key, _)| key.clone())
        .min()
}

/// The `peer_groups` key whose grant actually authorizes `identity` for
/// `<channel_type>.<alias>`.
#[must_use]
pub fn authorizing_group_key(
    cfg: &Config,
    channel_type: &str,
    alias: &str,
    identity: &str,
    match_fn: impl Fn(&str, &str) -> bool,
) -> Option<String> {
    let dotted_ref = format!("{channel_type}.{alias}");
    cfg.peer_groups
        .iter()
        .filter(|(_, group)| {
            let channel = group.channel.as_str();
            channel == dotted_ref || channel == channel_type
        })
        .filter(|(_, group)| {
            let grants: Vec<String> = group
                .external_peers
                .iter()
                .map(|peer| peer.as_str().to_string())
                .collect();
            crate::allowlist::is_user_allowed_by(&grants, identity, &match_fn)
        })
        .map(|(key, _)| key.clone())
        .min()
}

/// Merge `identity` into the `external_peers` of the peer group whose
/// `channel` ref matches `<channel_type>.<alias>`, on the canonical
/// in-memory config.
pub fn merge_external_peer(
    cfg: &mut Config,
    channel_type: &str,
    alias: &str,
    identity: &str,
    match_fn: impl Fn(&str, &str) -> bool,
) -> anyhow::Result<Option<String>> {
    use galleon_config::multi_agent::{PeerGroupConfig, PeerUsername};
    use galleon_config::providers::ChannelRef;

    let normalized = identity.trim();
    if normalized.is_empty() {
        anyhow::bail!("Cannot persist empty {channel_type} identity");
    }
    let configured = cfg
        .channels_by_alias()
        .iter()
        .any(|info| info.channel_type == channel_type && info.alias == alias);
    if !configured {
        anyhow::bail!(
            "Missing [channels.{channel_type}.{alias}] section in config.toml — \
             configure the channel before pairing"
        );
    }

    let resolved = cfg.channel_external_peers(channel_type, alias);

    if let Some(conflict) =
        external_peer_deny_conflict(cfg, channel_type, alias, &[normalized], &match_fn)
    {
        anyhow::bail!(conflict);
    }

    if crate::allowlist::is_user_allowed_by(&resolved, normalized, &match_fn) {
        return Ok(None);
    }

    let dotted_ref = format!("{channel_type}.{alias}");
    let conventional_key = format!("{channel_type}_{alias}");

    let target_key = instance_group_key(cfg, channel_type, alias);

    if let Some(key) = target_key {
        if let Some(group) = cfg.peer_groups.get_mut(&key) {
            group
                .external_peers
                .push(PeerUsername::new(normalized.to_string()));
        }
        return Ok(Some(key));
    }

    if let Some(existing) = cfg.peer_groups.get(&conventional_key) {
        anyhow::bail!(
            "peer group [{conventional_key}] already exists but its channel ref \
             is `{}` (expected `{dotted_ref}`) — fix the group key or channel ref \
             in config.toml before pairing",
            existing.channel.as_str()
        );
    }
    cfg.peer_groups.insert(
        conventional_key.clone(),
        PeerGroupConfig {
            channel: ChannelRef::new(dotted_ref),
            external_peers: vec![PeerUsername::new(normalized.to_string())],
            ..PeerGroupConfig::default()
        },
    );
    Ok(Some(conventional_key))
}

/// Persist a paired identity as an authorized external peer.
pub async fn persist_external_peer(
    persist: Option<&std::sync::Arc<parking_lot::RwLock<Config>>>,
    channel_type: &str,
    alias: &str,
    identity: &str,
    match_fn: impl Fn(&str, &str) -> bool,
) -> anyhow::Result<()> {
    use anyhow::Context;

    let Some(config) = persist else {
        ::galleon_log::record!(
            WARN,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                .with_outcome(::galleon_log::EventOutcome::Unknown)
                .with_attrs(::serde_json::json!({
                    "channel_type": channel_type,
                    "alias": alias,
                })),
            "paired identity not persisted (no persistence handle wired)"
        );
        return Ok(());
    };
    let _write_guard = galleon_config::write_lock::shared_config_write_lock()
        .lock_owned()
        .await;
    let mut staged = config.read().clone();
    if let Some(persisted) =
        galleon_config::schema::persisted_peer_groups(&staged.config_path).await?
    {
        staged.peer_groups = persisted;
    }
    if merge_external_peer(&mut staged, channel_type, alias, identity, &match_fn)?.is_none() {
        return Ok(());
    }
    staged.mark_dirty("peer_groups");
    staged
        .save_dirty()
        .await
        .with_context(|| format!("Failed to persist {channel_type} peer to config.toml"))?;
    {
        let mut cfg = config.write();
        merge_external_peer(&mut cfg, channel_type, alias, identity, &match_fn)?;
    }
    Ok(())
}
