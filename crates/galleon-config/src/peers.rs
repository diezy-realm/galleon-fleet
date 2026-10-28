//! Peer-group resolution and channel prompt mapping.

use std::collections::{BTreeMap, BTreeSet};
use crate::schema::Config;

const MAX_PEER_MAP_CHANNELS: usize = 10;
const MAX_PEER_MAP_ITEMS: usize = 20;
const MAX_PEER_MAP_CHARS: usize = 4_000;
const MAX_PROMPT_VALUE_CHARS: usize = 80;

/// Effective peer set for one agent, keyed by channel type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResolvedPeers {
    /// Channel type → peer-agent aliases (bound agent excluded).
    pub agent_peers: BTreeMap<String, BTreeSet<String>>,
    /// Channel type → external-peer usernames (case-folded).
    pub external_peers: BTreeMap<String, BTreeSet<String>>,
}

impl ResolvedPeers {
    /// Whether the bound agent recognizes `target` as a peer on a
    /// channel of `channel_type`. Outbound gate: unknown returns false.
    #[must_use]
    pub fn is_known_peer(&self, channel_type: &str, target: &str) -> bool {
        let normalized = target.trim_start_matches('@').to_ascii_lowercase();
        if let Some(agent_set) = self.agent_peers.get(channel_type)
            && agent_set.contains(&normalized)
        {
            return true;
        }
        if let Some(ext_set) = self.external_peers.get(channel_type)
            && ext_set.contains(&normalized)
        {
            return true;
        }
        false
    }

    #[must_use]
    pub fn allows_inbound(&self, channel_type: &str, origin: &str) -> bool {
        let normalized = origin.trim_start_matches('@').to_ascii_lowercase();
        if let Some(agent_set) = self.agent_peers.get(channel_type)
            && agent_set.contains(&normalized)
        {
            return true;
        }
        if let Some(ext_set) = self.external_peers.get(channel_type)
            && ext_set.contains(&normalized)
        {
            return true;
        }
        true
    }
}

#[must_use]
pub fn should_drop_self_loop(sender: &str, self_handle: Option<&str>) -> bool {
    let Some(handle) = self_handle else {
        return false;
    };
    let handle_norm = handle.trim_start_matches('@').to_ascii_lowercase();
    let sender_norm = sender.trim_start_matches('@').to_ascii_lowercase();
    !handle_norm.is_empty() && handle_norm == sender_norm
}

#[must_use]
pub fn resolve_peer_set(config: &Config, agent_alias: &str) -> ResolvedPeers {
    let mut resolved = ResolvedPeers::default();

    for group in config.peer_groups.values() {
        let on_group = group.agents.iter().any(|a| a.as_str() == agent_alias);
        if !on_group {
            continue;
        }

        let channel = group.channel.to_string();
        let agent_set = resolved.agent_peers.entry(channel.clone()).or_default();
        let self_norm = agent_alias.trim_start_matches('@').to_ascii_lowercase();
        for member in &group.agents {
            let normalized = member.as_str().trim_start_matches('@').to_ascii_lowercase();
            if normalized != self_norm {
                agent_set.insert(normalized);
            }
        }

        let ext_set = resolved.external_peers.entry(channel.clone()).or_default();
        for ext in &group.external_peers {
            ext_set.insert(ext.as_str().trim_start_matches('@').to_ascii_lowercase());
        }

        for ignored in &group.ignore {
            let needle = ignored
                .as_str()
                .trim_start_matches('@')
                .to_ascii_lowercase();
            ext_set.remove(&needle);
            agent_set.remove(&needle);
        }
    }

    resolved
}

#[must_use]
pub fn render_sender_peer_map_for_channel(
    config: &Config,
    sender_alias: &str,
    current_channel_ref: &str,
) -> String {
    let Some(agent) = config.agents.get(sender_alias) else {
        return String::new();
    };

    let resolved = resolve_peer_set(config, sender_alias);
    let agent_channels: Vec<&str> = agent.channels.iter().map(|c| c.as_str()).collect();
    let channel_keys: std::collections::BTreeSet<String> = resolved
        .agent_peers
        .keys()
        .chain(resolved.external_peers.keys())
        .filter(|channel_key| channel_matches_group(current_channel_ref, channel_key))
        .cloned()
        .collect();
    if channel_keys.is_empty() {
        return String::new();
    }

    let total_channels = channel_keys.len();
    let shown_channels: Vec<String> = channel_keys
        .iter()
        .take(MAX_PEER_MAP_CHANNELS)
        .cloned()
        .collect();

    let mut rows = Vec::new();
    for channel_key in shown_channels {
        let channel_refs =
            matching_current_channel_refs(&agent_channels, &channel_key, current_channel_ref);
        let channel_hint = if channel_refs.is_empty() {
            format!(
                "current channel ref {} is not listed in this agent's configured channels",
                prompt_value(current_channel_ref)
            )
        } else {
            format!("use channel ref {}", format_prompt_list(&channel_refs))
        };

        let group_names = group_names_for_channel(config, sender_alias, &channel_key);
        let agent_peers = agent_peers_for_current_channel(
            config,
            resolved.agent_peers.get(&channel_key),
            current_channel_ref,
        );
        let external_peers = set_values(resolved.external_peers.get(&channel_key));

        rows.push(format!(
            "- Channel scope {}: {channel_hint}; peer groups: {}; agent peers: {}; external peers: {}.",
            prompt_value(&channel_key),
            format_prompt_list(&group_names),
            format_prompt_list(&agent_peers),
            format_prompt_list(&external_peers)
        ));
    }

    let mut omitted_channels = total_channels.saturating_sub(MAX_PEER_MAP_CHANNELS);
    let mut peer_map = format_peer_map(sender_alias, &rows, omitted_channels);
    while peer_map.len() > MAX_PEER_MAP_CHARS && !rows.is_empty() {
        rows.pop();
        omitted_channels += 1;
        peer_map = format_peer_map(sender_alias, &rows, omitted_channels);
    }

    peer_map
}

fn format_peer_map(sender_alias: &str, rows: &[String], omitted_channels: usize) -> String {
    let mut row_text = rows.join("\n");
    if omitted_channels > 0 {
        if !row_text.is_empty() {
            row_text.push('\n');
        }
        row_text.push_str(&format!(
            "- {omitted_channels} more channel scopes omitted from this prompt map."
        ));
    }

    format!(
        "Current-channel peer map for agent {}:\n{}\n\
         For collective wording like \"the whole team\" or a peer group name, send only to the listed agent peers unless the user explicitly names external peers. \
         Do not pass peer group names as the `channel` parameter; pass one of the listed channel refs.",
        prompt_value(sender_alias),
        row_text
    )
}

fn group_names_for_channel(config: &Config, sender_alias: &str, channel_key: &str) -> Vec<String> {
    let mut names: Vec<String> = config
        .peer_groups
        .iter()
        .filter(|(_, group)| {
            group.channel == channel_key
                && group
                    .agents
                    .iter()
                    .any(|member| member.as_str() == sender_alias)
        })
        .map(|(name, _)| name.clone())
        .collect();
    names.sort();
    names
}

fn set_values(set: Option<&std::collections::BTreeSet<String>>) -> Vec<String> {
    set.map(|set| set.iter().cloned().collect())
        .unwrap_or_default()
}

fn agent_peers_for_current_channel(
    config: &Config,
    set: Option<&std::collections::BTreeSet<String>>,
    current_channel_ref: &str,
) -> Vec<String> {
    set.map(|set| {
        set.iter()
            .filter(|alias| agent_listens_on_channel_ref(config, alias, current_channel_ref))
            .cloned()
            .collect()
    })
    .unwrap_or_default()
}

fn agent_listens_on_channel_ref(config: &Config, alias: &str, channel_ref: &str) -> bool {
    config
        .agents
        .iter()
        .find(|(configured_alias, _)| configured_alias.eq_ignore_ascii_case(alias))
        .is_some_and(|(_, agent)| agent.channels.iter().any(|channel| channel == channel_ref))
}

fn matching_current_channel_refs(
    agent_channels: &[&str],
    group_channel: &str,
    current_channel_ref: &str,
) -> Vec<String> {
    if !channel_matches_group(current_channel_ref, group_channel) {
        return Vec::new();
    }

    if agent_channels.contains(&current_channel_ref) {
        return vec![current_channel_ref.to_string()];
    }

    Vec::new()
}

fn channel_matches_group(channel_ref: &str, group_channel: &str) -> bool {
    if group_channel.contains('.') {
        return channel_ref == group_channel;
    }
    channel_ref == group_channel
        || channel_ref
            .split_once('.')
            .map(|(channel_type, _)| channel_type == group_channel)
            .unwrap_or(false)
}

fn prompt_value(value: &str) -> String {
    let mut chars = value.chars();
    let mut value: String = chars.by_ref().take(MAX_PROMPT_VALUE_CHARS).collect();
    if chars.next().is_some() {
        value.push_str("...");
    }
    serde_json::to_string(&value).unwrap_or_else(|_| "\"<invalid>\"".to_string())
}

fn format_prompt_list(values: &[String]) -> String {
    if values.is_empty() {
        return "none".to_string();
    }

    let mut parts: Vec<String> = values
        .iter()
        .take(MAX_PEER_MAP_ITEMS)
        .map(|value| prompt_value(value))
        .collect();
    if values.len() > MAX_PEER_MAP_ITEMS {
        parts.push(format!("and {} more", values.len() - MAX_PEER_MAP_ITEMS));
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_drop_self_loop_returns_false_when_handle_unknown() {
        assert!(!should_drop_self_loop("@anyone", None));
    }

    #[test]
    fn should_drop_self_loop_matches_normalized_handle() {
        assert!(should_drop_self_loop("@my_bot", Some("@my_bot")));
        assert!(should_drop_self_loop("@MY_BOT", Some("my_bot")));
        assert!(should_drop_self_loop("my_bot", Some("@My_Bot")));
        assert!(!should_drop_self_loop("@other_bot", Some("@my_bot")));
    }

    #[test]
    fn should_drop_self_loop_ignores_empty_handle_after_normalization() {
        assert!(!should_drop_self_loop("@anyone", Some("@")));
    }
}
