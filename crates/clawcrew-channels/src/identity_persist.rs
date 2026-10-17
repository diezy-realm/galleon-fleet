//! Shared paired-identity persistence for QR-pairing channels.
//!
//! When a QR pairing completes, the linked account identity becomes an
//! authorized external peer. The canonical home for that authorization is
//! a `[peer_groups.<name>]` entry in `config.toml` whose `channel` field
//! is the dotted `<channel_type>.<alias>` instance ref — the same
//! channel-ref contract `Config::channel_external_peers` matches at
//! message-time (the runtime reader never looks at the map key). This
//! module is the single writer for that shape: WeChat, WhatsApp Web,
//! Telegram and LINE all persist through it, so no two channels can drift
//! into different on-disk layouts (and no channel grows a local allowlist
//! cache).
//!
//! Writes go through the shared `Arc<RwLock<Config>>` handle the
//! orchestrator wires into each channel — stage the merge on a clone, save
//! it, and publish to the canonical in-memory state only once the durable
//! write has succeeded, so a failed save cannot leave a grant the file never
//! received. Channels
//! constructed without the handle (tests, one-shot CLI runs) skip
//! persistence with a warning: pairing still works for the process
//! lifetime, it just isn't durable.

#[allow(unused_imports)]
pub use galleon_channel_core::identity_persist::{
    authorizing_group_key, external_peer_deny_conflict, instance_group_key,
    merge_external_peer, persist_external_peer,
};

#[cfg(test)]
mod tests {
    use super::*;
    use clawcrew_config::schema::Config;
    use std::sync::Arc;

    /// Stands in for a channel whose identities are compared verbatim, for the
    /// group-selection tests where the matcher is not what is under test.
    fn exact(entry: &str, user: &str) -> bool {
        entry == user
    }

    fn config_with_whatsapp(alias: &str) -> Config {
        let mut config = Config::default();
        config.channels.whatsapp.insert(
            alias.to_string(),
            clawcrew_config::schema::WhatsAppConfig {
                enabled: true,
                ..Default::default()
            },
        );
        config
    }

    #[test]
    fn merge_creates_group_in_the_wechat_shape() {
        let mut config = config_with_whatsapp("admin");

        let changed = merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
            .expect("merge succeeds");
        assert!(changed.is_some());

        let group = config
            .peer_groups
            .get("whatsapp_admin")
            .expect("group created under <type>_<alias>");
        assert_eq!(group.channel.as_str(), "whatsapp.admin");
        assert_eq!(
            group
                .external_peers
                .iter()
                .map(|p| p.as_str().to_string())
                .collect::<Vec<_>>(),
            vec!["+15551234567".to_string()]
        );
    }

    #[test]
    fn merge_is_idempotent_and_additive() {
        let mut config = config_with_whatsapp("admin");

        assert!(
            merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
                .unwrap()
                .is_some()
        );
        assert!(
            merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
                .unwrap()
                .is_none(),
            "an already-authorized identity is not re-added"
        );
        assert!(
            merge_external_peer(&mut config, "whatsapp", "admin", "+15559876543", exact)
                .unwrap()
                .is_some(),
            "a second identity extends the same group"
        );
        assert_eq!(
            config
                .peer_groups
                .get("whatsapp_admin")
                .unwrap()
                .external_peers
                .len(),
            2
        );
    }

    #[test]
    fn merge_preserves_existing_group_membership() {
        use clawcrew_config::multi_agent::{AgentAlias, PeerGroupConfig, PeerUsername};
        use clawcrew_config::providers::ChannelRef;

        let mut config = config_with_whatsapp("admin");
        config.peer_groups.insert(
            "whatsapp_admin".to_string(),
            PeerGroupConfig {
                channel: ChannelRef::new("whatsapp.admin".to_string()),
                agents: vec![AgentAlias::new("rowan".to_string())],
                external_peers: vec![PeerUsername::new("+15550000000".to_string())],
                ..Default::default()
            },
        );

        assert!(
            merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
                .unwrap()
                .is_some()
        );
        let group = config.peer_groups.get("whatsapp_admin").unwrap();
        assert_eq!(group.agents.len(), 1, "agent bindings survive the merge");
        assert_eq!(group.external_peers.len(), 2);
    }

    #[test]
    fn merge_rejects_empty_identity_and_unconfigured_channel() {
        let mut config = config_with_whatsapp("admin");
        assert!(merge_external_peer(&mut config, "whatsapp", "admin", "  ", exact).is_err());
        assert!(
            merge_external_peer(&mut config, "whatsapp", "ghost", "+15551234567", exact).is_err(),
            "an alias with no [channels.whatsapp.ghost] block is rejected"
        );
        assert!(
            merge_external_peer(&mut config, "telegram", "admin", "someone", exact).is_err(),
            "a type/alias pair with no configured block is rejected"
        );
        assert!(
            config.peer_groups.is_empty(),
            "failed merges must not leave partial groups behind"
        );
    }

    #[test]
    fn merge_accepts_any_configured_channel_type_via_the_registry() {
        // The existence check reads the canonical channel registry, not a
        // hardcoded channel-type list: a configured channel of any type can
        // persist a paired identity without this module needing an edit.
        let mut config = Config::default();
        config.channels.telegram.insert(
            "admin".to_string(),
            clawcrew_config::schema::TelegramConfig {
                enabled: true,
                ..Default::default()
            },
        );

        assert!(
            merge_external_peer(&mut config, "telegram", "admin", "someone", exact)
                .unwrap()
                .is_some()
        );
        let group = config
            .peer_groups
            .get("telegram_admin")
            .expect("group created under <type>_<alias>");
        assert_eq!(group.channel.as_str(), "telegram.admin");
    }

    #[test]
    fn merge_surfaces_a_conflict_instead_of_persisting_a_shadowed_grant() {
        use clawcrew_config::multi_agent::{PeerGroupConfig, PeerUsername};
        use clawcrew_config::providers::ChannelRef;

        // The end of the recovery path: pairing is available again once a
        // fully shadowed grant no longer counts as authorization, so it must
        // not dead-end by appending a grant the deny immediately shadows.
        let mut config = config_with_whatsapp("admin");
        config.peer_groups.insert(
            "whatsapp_admin".to_string(),
            PeerGroupConfig {
                channel: ChannelRef::new("whatsapp.admin".to_string()),
                external_peers: vec![PeerUsername::new("+15551234567".to_string())],
                ignore: vec![PeerUsername::new("+15551234567".to_string())],
                ..Default::default()
            },
        );

        let err = merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
            .expect_err("an ignored identity must not be persisted as a grant");
        let message = err.to_string();
        assert!(
            message.contains("ignore"),
            "names the field to edit: {message}"
        );
        assert!(
            !message.contains("+15551234567"),
            "the identity is personal data and callers log this error: {message}"
        );

        let group = config.peer_groups.get("whatsapp_admin").unwrap();
        assert_eq!(
            group.external_peers.len(),
            1,
            "no second, equally shadowed grant was appended"
        );
        assert!(
            !crate::allowlist::is_user_allowed_by(
                &config.channel_external_peers("whatsapp", "admin"),
                "+15551234567",
                exact,
            ),
            "the operator's ignore stays authoritative"
        );

        // With the ignore removed, the same pairing attempt succeeds and the
        // identity is genuinely admissible: the recovery path terminates.
        config
            .peer_groups
            .get_mut("whatsapp_admin")
            .unwrap()
            .ignore
            .clear();
        assert!(
            merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
                .expect("merge succeeds once the deny is gone")
                .is_none(),
            "the existing grant is now effective, so nothing needs writing"
        );
        assert!(crate::allowlist::is_user_allowed_by(
            &config.channel_external_peers("whatsapp", "admin"),
            "+15551234567",
            exact,
        ));
    }

    #[test]
    fn merge_rejects_conventional_key_with_mismatched_channel_ref() {
        use clawcrew_config::multi_agent::{PeerGroupConfig, PeerUsername};
        use clawcrew_config::providers::ChannelRef;

        // A stale/hand-edited [peer_groups.whatsapp_admin] that points at a
        // different channel must not silently receive the WhatsApp identity:
        // whatsapp.admin's reader would never see it, telegram.admin's would.
        let mut config = config_with_whatsapp("admin");
        config.peer_groups.insert(
            "whatsapp_admin".to_string(),
            PeerGroupConfig {
                channel: ChannelRef::new("telegram.admin".to_string()),
                external_peers: vec![PeerUsername::new("someone".to_string())],
                ..Default::default()
            },
        );

        let err = merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
            .expect_err("mismatched channel ref must be rejected");
        assert!(err.to_string().contains("telegram.admin"));

        let group = config.peer_groups.get("whatsapp_admin").unwrap();
        assert_eq!(group.channel.as_str(), "telegram.admin", "group untouched");
        assert_eq!(group.external_peers.len(), 1, "no identity appended");
        assert!(
            config
                .channel_external_peers("whatsapp", "admin")
                .is_empty(),
            "the identity must not be stored anywhere the reader matches"
        );
    }

    #[test]
    fn merge_targets_group_by_channel_ref_not_map_key() {
        use clawcrew_config::multi_agent::PeerGroupConfig;
        use clawcrew_config::providers::ChannelRef;

        // The reader authorizes by the group's `channel` field, so the
        // writer appends to the group carrying the dotted ref even when it
        // lives under a non-conventional key — and creates no second group.
        let mut config = config_with_whatsapp("admin");
        config.peer_groups.insert(
            "my_custom_group".to_string(),
            PeerGroupConfig {
                channel: ChannelRef::new("whatsapp.admin".to_string()),
                ..Default::default()
            },
        );

        assert!(
            merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
                .unwrap()
                .is_some()
        );
        assert!(
            !config.peer_groups.contains_key("whatsapp_admin"),
            "no duplicate conventional group is created"
        );
        assert_eq!(
            config
                .peer_groups
                .get("my_custom_group")
                .unwrap()
                .external_peers
                .len(),
            1
        );
        assert_eq!(
            config.channel_external_peers("whatsapp", "admin"),
            vec!["+15551234567".to_string()],
            "the reader sees the persisted identity"
        );
    }

    #[test]
    fn merge_appends_to_matching_group_even_when_conventional_key_is_taken() {
        use clawcrew_config::multi_agent::{PeerGroupConfig, PeerUsername};
        use clawcrew_config::providers::ChannelRef;

        // The mismatched conventional key only blocks *creation*. When some
        // other group already carries this channel's dotted ref, the append
        // goes there and the foreign group is left untouched.
        let mut config = config_with_whatsapp("admin");
        config.peer_groups.insert(
            "whatsapp_admin".to_string(),
            PeerGroupConfig {
                channel: ChannelRef::new("telegram.admin".to_string()),
                external_peers: vec![PeerUsername::new("someone".to_string())],
                ..Default::default()
            },
        );
        config.peer_groups.insert(
            "renamed_whatsapp_group".to_string(),
            PeerGroupConfig {
                channel: ChannelRef::new("whatsapp.admin".to_string()),
                ..Default::default()
            },
        );

        assert!(
            merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            config
                .peer_groups
                .get("renamed_whatsapp_group")
                .unwrap()
                .external_peers
                .iter()
                .map(|p| p.as_str().to_string())
                .collect::<Vec<_>>(),
            vec!["+15551234567".to_string()],
            "identity lands in the group whose channel ref matches"
        );
        let foreign = config.peer_groups.get("whatsapp_admin").unwrap();
        assert_eq!(foreign.channel.as_str(), "telegram.admin");
        assert_eq!(
            foreign.external_peers.len(),
            1,
            "the foreign group under the conventional key is untouched"
        );
        assert_eq!(
            config.channel_external_peers("whatsapp", "admin"),
            vec!["+15551234567".to_string()],
            "the reader authorizes the identity for this channel"
        );
    }

    #[test]
    fn merge_prefers_the_conventional_key_when_several_groups_match() {
        use clawcrew_config::multi_agent::PeerGroupConfig;
        use clawcrew_config::providers::ChannelRef;

        // Two instance-scoped groups both carry `whatsapp.admin`; the append
        // deterministically targets the conventional `<type>_<alias>` key.
        let mut config = config_with_whatsapp("admin");
        for key in ["whatsapp_admin", "another_group"] {
            config.peer_groups.insert(
                key.to_string(),
                PeerGroupConfig {
                    channel: ChannelRef::new("whatsapp.admin".to_string()),
                    ..Default::default()
                },
            );
        }

        assert!(
            merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
                .unwrap()
                .is_some()
        );
        assert_eq!(
            config
                .peer_groups
                .get("whatsapp_admin")
                .unwrap()
                .external_peers
                .len(),
            1,
            "conventional key receives the append"
        );
        assert!(
            config
                .peer_groups
                .get("another_group")
                .unwrap()
                .external_peers
                .is_empty(),
            "the other matching group is untouched"
        );
    }

    #[test]
    fn merge_is_a_noop_when_a_type_wide_group_already_authorizes() {
        use clawcrew_config::multi_agent::{PeerGroupConfig, PeerUsername};
        use clawcrew_config::providers::ChannelRef;

        // A bare-type group (`channel = "whatsapp"`) already authorizes the
        // identity for every alias of the type; the reader-driven
        // idempotency check must catch that and write nothing.
        let mut config = config_with_whatsapp("admin");
        config.peer_groups.insert(
            "whatsapp_everyone".to_string(),
            PeerGroupConfig {
                channel: ChannelRef::new("whatsapp".to_string()),
                external_peers: vec![PeerUsername::new("+15551234567".to_string())],
                ..Default::default()
            },
        );

        assert!(
            merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
                .unwrap()
                .is_none(),
            "already authorized via the type-wide group"
        );
        assert_eq!(config.peer_groups.len(), 1, "no new group created");
    }

    #[test]
    fn merge_does_not_treat_an_ignored_identity_as_already_authorized() {
        use clawcrew_config::multi_agent::{PeerGroupConfig, PeerUsername};
        use clawcrew_config::providers::ChannelRef;

        // The resolved list carries grants and denies together, so a grant this
        // identity matches can sit beside an `ignore` that denies it. Comparing
        // the entries by string would find the grant and report the identity as
        // already authorized, which is the opposite of the truth. The deny
        // reaches here through a type-wide group, not the instance-scoped one.
        let mut config = config_with_whatsapp("admin");
        config.peer_groups.insert(
            "whatsapp_everyone".to_string(),
            PeerGroupConfig {
                channel: ChannelRef::new("whatsapp".to_string()),
                external_peers: vec![PeerUsername::new("+15551234567".to_string())],
                ignore: vec![PeerUsername::new("+15551234567".to_string())],
                ..Default::default()
            },
        );

        // Neither of the two silent outcomes is right: `Ok(false)` would claim
        // the identity is already authorized, and `Ok(true)` would append a
        // grant the deny shadows just as thoroughly.
        merge_external_peer(&mut config, "whatsapp", "admin", "+15551234567", exact)
            .expect_err("an ignored identity is neither authorized nor grantable by appending");
        assert_eq!(
            config.peer_groups.len(),
            1,
            "no group was created to hold a shadowed grant"
        );
    }

    #[cfg(any(feature = "channel-wechat", feature = "whatsapp-web"))]
    #[tokio::test]
    async fn persist_without_handle_warns_and_returns_ok() {
        persist_external_peer(None, "whatsapp", "admin", "+15551234567", exact)
            .await
            .expect("missing handle is a soft no-op");
    }

    /// A failed `Config::save()` must leave nothing behind: not on disk, and
    /// not in the live config either.
    ///
    /// Mutating the shared config before the save made the grant survive a
    /// failure the file never recorded. The retry then read its own leftover
    /// through the already-authorized check, returned `Ok` without attempting
    /// another write, and the channel answered "bound" for an authorization
    /// that vanished on restart.
    #[tokio::test]
    async fn failed_save_leaves_no_grant_and_the_retry_still_persists() {
        let dir = tempfile::tempdir().expect("tempdir");
        // The config's parent directory is an existing regular file, so
        // `create_dir_all` in the atomic writer fails on every platform.
        // A path under a non-existent root does not: `/nonexistent-.../` is
        // creatable on the Windows runner, so the expected save error never
        // occurred there and the test failed for the wrong reason.
        let blocker = dir.path().join("not-a-directory");
        std::fs::write(&blocker, b"").expect("create the blocking file");
        let config_path = blocker.join("config.toml");
        let mut config = config_with_whatsapp("admin");
        config.config_path = config_path.clone();
        let shared = Arc::new(parking_lot::RwLock::new(config));

        let first =
            persist_external_peer(Some(&shared), "whatsapp", "admin", "+15551234567", exact).await;
        assert!(first.is_err(), "an unwritable config path must surface");

        assert!(
            shared
                .read()
                .channel_external_peers("whatsapp", "admin")
                .is_empty(),
            "a save that failed must not leave the grant live in memory"
        );

        // The retry must reach the writer again rather than short-circuit on a
        // leftover grant. Under the old ordering this returned Ok(()), which is
        // the channel reporting a successful binding that was never persisted.
        let second =
            persist_external_peer(Some(&shared), "whatsapp", "admin", "+15551234567", exact).await;
        assert!(
            second.is_err(),
            "the retry must attempt persistence again, not report success"
        );
        assert!(
            shared
                .read()
                .channel_external_peers("whatsapp", "admin")
                .is_empty(),
            "still nothing authorized after the second failure"
        );

        // Repointed at a writable path the same call succeeds, so the two
        // failures above came from the unwritable location and not from a
        // writer that could never have persisted anything.
        shared.write().config_path = dir.path().join("config.toml");
        persist_external_peer(Some(&shared), "whatsapp", "admin", "+15551234567", exact)
            .await
            .expect("the retry persists once the path is writable");
        assert_eq!(
            shared.read().channel_external_peers("whatsapp", "admin"),
            vec!["+15551234567".to_string()]
        );
    }

    /// An identity already authorized by a bare type-wide group must be
    /// reported against that group, not the conventional instance key.
    ///
    /// `instance_group_key` only matches exact dotted refs, so the bind
    /// endpoint fell back to `<type>_<alias>` and named a block that need not
    /// exist. An operator following that answer edits the wrong group.
    #[test]
    fn authorizing_group_key_names_a_type_wide_source() {
        use clawcrew_config::multi_agent::{PeerGroupConfig, PeerUsername};
        use clawcrew_config::providers::ChannelRef;

        let mut config = config_with_whatsapp("admin");
        config.peer_groups.insert(
            "whatsapp_all".to_string(),
            PeerGroupConfig {
                channel: ChannelRef::new("whatsapp".to_string()),
                external_peers: vec![PeerUsername::new("+15551234567".to_string())],
                ..Default::default()
            },
        );

        assert_eq!(
            authorizing_group_key(&config, "whatsapp", "admin", "+15551234567", exact).as_deref(),
            Some("whatsapp_all"),
            "the group carrying the grant is the one to name"
        );
        assert_eq!(
            instance_group_key(&config, "whatsapp", "admin"),
            None,
            "and the write-target resolver genuinely cannot answer this"
        );

        // An identity no group grants has no source to report.
        assert_eq!(
            authorizing_group_key(&config, "whatsapp", "admin", "+15559999999", exact),
            None
        );
    }

    /// A deny written by a competing config writer must survive a pairing bind,
    /// on disk and not only in memory.
    ///
    /// `Config::save` syncs the staged snapshot onto the existing document and
    /// drops keys the snapshot does not carry, so pairing's save used to write
    /// the pre-deny policy back over the file. Publishing the merge into the
    /// live config kept the deny in memory and could not undo the file, so the
    /// next load authorized the account the operator had just denied.
    ///
    /// The interleaving is forced rather than raced: the test holds the shared
    /// config write lock the way a gateway handler does, so the spawned pairing
    /// transaction cannot take its snapshot until the competing `ignore` has
    /// been published and saved.
    #[tokio::test]
    async fn a_competing_ignore_survives_a_pairing_write() {
        use clawcrew_config::multi_agent::{PeerGroupConfig, PeerUsername};
        use clawcrew_config::providers::ChannelRef;

        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let mut config = config_with_whatsapp("admin");
        config.config_path = config_path.clone();
        config.save().await.expect("seed the config file");
        let shared = Arc::new(parking_lot::RwLock::new(config));

        let guard = clawcrew_config::write_lock::shared_config_write_lock()
            .lock_owned()
            .await;

        let pairing = {
            let shared = Arc::clone(&shared);
            clawcrew_spawn::spawn!(async move {
                persist_external_peer(Some(&shared), "whatsapp", "admin", "+15551234567", exact)
                    .await
            })
        };

        // Let the task reach the lock and block there.
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }
        assert!(
            !pairing.is_finished(),
            "pairing must not run its transaction while another writer holds the lock"
        );

        // The competing writer, in the shape the gateway uses: mutate the live
        // config, then save the whole snapshot.
        {
            let mut cfg = shared.write();
            cfg.peer_groups.insert(
                "whatsapp_admin_denies".to_string(),
                PeerGroupConfig {
                    channel: ChannelRef::new("whatsapp.admin".to_string()),
                    ignore: vec![PeerUsername::new("+15559999999".to_string())],
                    ..Default::default()
                },
            );
        }
        let competing = shared.read().clone();
        competing.save().await.expect("the competing save lands");

        drop(guard);
        pairing
            .await
            .expect("the pairing task joins")
            .expect("pairing persists once the lock is free");

        let on_disk: Config = toml::from_str(
            &std::fs::read_to_string(&config_path).expect("config.toml is readable"),
        )
        .expect("config.toml round-trips");

        let peers = on_disk.channel_external_peers("whatsapp", "admin");
        assert!(
            peers.iter().any(|p| p == "!+15559999999"),
            "the competing deny must still be on disk after pairing saved: {peers:?}"
        );
        assert!(
            peers.iter().any(|p| p == "+15551234567"),
            "and pairing's own grant must have landed too: {peers:?}"
        );
    }

    /// The production split: the gateway/RPC copy of `Config` and the channel
    /// copy are *different* values over the same file, so serializing the
    /// writers is not enough. A deny saved through one handle is invisible to
    /// the other, and pairing used to check denies against that stale policy
    /// and then write it back over the file, erasing a security decision the
    /// operator had already saved.
    ///
    /// `a_competing_ignore_survives_a_pairing_write` cannot catch this: both of
    /// its writers share one `Arc<RwLock<Config>>`, so pairing's snapshot sees
    /// the deny for free. Here the channel handle never sees it, and the repair
    /// has to come from re-reading the persisted policy under the lock.
    #[tokio::test]
    async fn a_deny_saved_through_a_separate_handle_survives_and_blocks_the_bind() {
        use clawcrew_config::multi_agent::{PeerGroupConfig, PeerUsername};
        use clawcrew_config::providers::ChannelRef;

        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");

        let mut seed = config_with_whatsapp("admin");
        seed.config_path = config_path.clone();
        seed.save().await.expect("seed the config file");

        // Two independent handles over the same file, as the daemon builds them.
        let mut gateway = seed.clone();
        let channel = Arc::new(parking_lot::RwLock::new(seed.clone()));

        // The operator denies an account through the gateway and it lands.
        gateway.peer_groups.insert(
            "whatsapp_admin_denies".to_string(),
            PeerGroupConfig {
                channel: ChannelRef::new("whatsapp.admin".to_string()),
                ignore: vec![PeerUsername::new("+15559999999".to_string())],
                ..Default::default()
            },
        );
        gateway.save().await.expect("the deny save lands");

        // The channel handle is stale, which is the whole point.
        assert!(
            !channel
                .read()
                .peer_groups
                .contains_key("whatsapp_admin_denies"),
            "the channel handle must not have seen the gateway's write"
        );

        // Pairing the denied account must be refused, from the file rather than
        // from this handle's memory.
        let refused =
            persist_external_peer(Some(&channel), "whatsapp", "admin", "+15559999999", exact).await;
        assert!(
            refused.is_err(),
            "a denied identity must not bind through a stale handle"
        );

        // An unrelated account still pairs, and the deny must survive that write.
        persist_external_peer(Some(&channel), "whatsapp", "admin", "+15551234567", exact)
            .await
            .expect("an undenied bind still persists");

        let on_disk: Config = toml::from_str(
            &std::fs::read_to_string(&config_path).expect("config.toml is readable"),
        )
        .expect("config.toml round-trips");

        let peers = on_disk.channel_external_peers("whatsapp", "admin");
        assert!(
            peers.iter().any(|p| p == "!+15559999999"),
            "the deny saved through the other handle must still be on disk: {peers:?}"
        );
        assert!(
            peers.iter().any(|p| p == "+15551234567"),
            "and the undenied grant must have landed: {peers:?}"
        );
    }

    /// The other half of the same repair: pairing writes `peer_groups`
    /// incrementally, so a snapshot that predates another writer cannot drop
    /// the keys it never saw.
    ///
    /// Refreshing the policy alone would not cover this — the refresh only
    /// touches `peer_groups`, while a full `Config::save` rewrites the entire
    /// document from the stale snapshot.
    #[tokio::test]
    async fn a_pairing_write_through_a_stale_handle_keeps_keys_it_never_saw() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");

        let mut seed = config_with_whatsapp("admin");
        seed.config_path = config_path.clone();
        seed.save().await.expect("seed the config file");

        let mut gateway = seed.clone();
        let channel = Arc::new(parking_lot::RwLock::new(seed.clone()));

        // A second channel instance is configured through the gateway, outside
        // `peer_groups` entirely.
        gateway.channels.whatsapp.insert(
            "ops".to_string(),
            clawcrew_config::schema::WhatsAppConfig {
                enabled: true,
                ..Default::default()
            },
        );
        gateway.save().await.expect("the gateway save lands");

        assert!(
            !channel.read().channels.whatsapp.contains_key("ops"),
            "the channel handle must not have seen the new instance"
        );

        persist_external_peer(Some(&channel), "whatsapp", "admin", "+15551234567", exact)
            .await
            .expect("pairing persists");

        let on_disk: Config = toml::from_str(
            &std::fs::read_to_string(&config_path).expect("config.toml is readable"),
        )
        .expect("config.toml round-trips");

        assert!(
            on_disk.channels.whatsapp.contains_key("ops"),
            "pairing must not drop a key its snapshot predates"
        );
        assert!(
            on_disk
                .channel_external_peers("whatsapp", "admin")
                .iter()
                .any(|p| p == "+15551234567"),
            "and pairing's own grant must still have landed"
        );
    }

    /// A deny that lands mid-transaction and names the identity being paired
    /// must stop the bind, not be written over.
    ///
    /// `pairing_deny_conflict` already refused a deny it could see. It could
    /// not see this one: the snapshot was taken before the competing write, so
    /// the guard ran against stale state and the bind proceeded. Taking the
    /// snapshot under the write lock is what makes the existing guard effective
    /// against a concurrent writer.
    #[tokio::test]
    async fn a_competing_ignore_naming_the_paired_identity_blocks_the_bind() {
        use clawcrew_config::multi_agent::{PeerGroupConfig, PeerUsername};
        use clawcrew_config::providers::ChannelRef;

        let dir = tempfile::tempdir().expect("tempdir");
        let config_path = dir.path().join("config.toml");
        let mut config = config_with_whatsapp("admin");
        config.config_path = config_path.clone();
        config.save().await.expect("seed the config file");
        let shared = Arc::new(parking_lot::RwLock::new(config));

        let guard = clawcrew_config::write_lock::shared_config_write_lock()
            .lock_owned()
            .await;

        let pairing = {
            let shared = Arc::clone(&shared);
            clawcrew_spawn::spawn!(async move {
                persist_external_peer(Some(&shared), "whatsapp", "admin", "+15551234567", exact)
                    .await
            })
        };
        for _ in 0..8 {
            tokio::task::yield_now().await;
        }

        {
            let mut cfg = shared.write();
            cfg.peer_groups.insert(
                "whatsapp_admin_denies".to_string(),
                PeerGroupConfig {
                    channel: ChannelRef::new("whatsapp.admin".to_string()),
                    ignore: vec![PeerUsername::new("+15551234567".to_string())],
                    ..Default::default()
                },
            );
        }
        // Bound before the await: a `parking_lot` read guard must not be held
        // across one.
        let competing = shared.read().clone();
        competing.save().await.expect("the competing save lands");

        drop(guard);
        let err = pairing
            .await
            .expect("the pairing task joins")
            .expect_err("a denied identity must not be bound");
        assert!(
            err.to_string().contains("denied by an `ignore` entry"),
            "the operator is told what to edit: {err}"
        );

        let on_disk: Config = toml::from_str(
            &std::fs::read_to_string(&config_path).expect("config.toml is readable"),
        )
        .expect("config.toml round-trips");
        let peers = on_disk.channel_external_peers("whatsapp", "admin");
        assert!(
            !peers.iter().any(|p| p == "+15551234567"),
            "no grant for the denied identity reached disk: {peers:?}"
        );
    }

    /// The success path still publishes, so the fix cannot be "never write".
    #[tokio::test]
    async fn successful_save_publishes_the_grant_to_the_live_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut config = config_with_whatsapp("admin");
        config.config_path = dir.path().join("config.toml");
        let shared = Arc::new(parking_lot::RwLock::new(config));

        persist_external_peer(Some(&shared), "whatsapp", "admin", "+15551234567", exact)
            .await
            .expect("a writable path persists");

        assert_eq!(
            shared.read().channel_external_peers("whatsapp", "admin"),
            vec!["+15551234567".to_string()],
            "the live config carries the grant once the save succeeded"
        );
    }
}
