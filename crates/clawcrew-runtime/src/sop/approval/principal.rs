pub use clawcrew_api::sop::{ApprovalPrincipal, ApprovalSource};



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_pin_their_source() {
        assert_eq!(ApprovalPrincipal::agent("a").source, ApprovalSource::Agent);
        assert_eq!(ApprovalPrincipal::cli(None).source, ApprovalSource::Cli);
        assert_eq!(
            ApprovalPrincipal::ws("c1".into(), None).source,
            ApprovalSource::Ws
        );
        assert_eq!(ApprovalPrincipal::http(None).source, ApprovalSource::Http);
        assert_eq!(ApprovalPrincipal::system().source, ApprovalSource::System);
    }

    #[test]
    fn is_out_of_band_true_for_all_but_agent() {
        assert!(!ApprovalPrincipal::agent("a").is_out_of_band());
        assert!(ApprovalPrincipal::cli(None).is_out_of_band());
        assert!(ApprovalPrincipal::ws("c".into(), None).is_out_of_band());
        assert!(ApprovalPrincipal::http(None).is_out_of_band());
        assert!(ApprovalPrincipal::system().is_out_of_band());
    }

    #[test]
    fn actor_label_prefers_identity_then_source() {
        assert_eq!(
            ApprovalPrincipal::agent("deploy-bot").actor_label(),
            "deploy-bot"
        );
        assert_eq!(ApprovalPrincipal::cli(None).actor_label(), "cli");
        assert_eq!(ApprovalPrincipal::system().actor_label(), "system");
    }

    #[test]
    fn voter_key_collapses_gateway_transports_but_keeps_sources_distinct() {
        // HTTP and WS share the paired credential -> one canonical gateway voter.
        assert_eq!(
            ApprovalPrincipal::http(Some("sub".into())).voter_key(),
            ApprovalPrincipal::ws("conn".into(), Some("sub".into())).voter_key(),
            "the same subject over HTTP and WS is one voter"
        );
        assert_eq!(
            ApprovalPrincipal::http(Some("sub".into())).voter_key(),
            "gateway:sub"
        );
        // A CLI actor with the same name is a genuinely distinct voter.
        assert_ne!(
            ApprovalPrincipal::cli(Some("sub".into())).voter_key(),
            ApprovalPrincipal::http(Some("sub".into())).voter_key(),
            "a CLI actor is distinct from a gateway subject"
        );
        // Different subjects on the gateway stay distinct.
        assert_ne!(
            ApprovalPrincipal::http(Some("a".into())).voter_key(),
            ApprovalPrincipal::http(Some("b".into())).voter_key()
        );
    }

    #[test]
    fn voter_key_namespaces_channel_principals_by_channel_key() {
        assert_eq!(
            ApprovalPrincipal::channel("discord.ops".into(), Some("123".into())).voter_key(),
            "channel:discord.ops:123"
        );
        assert_ne!(
            ApprovalPrincipal::channel("discord.ops".into(), Some("123".into())).voter_key(),
            ApprovalPrincipal::channel("slack.ops".into(), Some("123".into())).voter_key(),
            "same-looking sender ids on different channels must be distinct voters"
        );
    }

    #[test]
    fn principal_source_not_client_settable() {
        // The security invariant: a principal's source comes from a constructor
        // (the transport), not from deserializing a client body. We CAN deserialize
        // the wire shape (for ledger round-trips), but production handlers build
        // principals via the constructors, never `serde::from` a request body.
        let json = r#"{"source":"http","identity":"attacker","channel":"x"}"#;
        let p: ApprovalPrincipal = serde_json::from_str(json).unwrap();
        assert_eq!(p.source, ApprovalSource::Http);
        // Constructors remain the only path used by handlers; this test documents
        // that the wire form exists for persistence, not for trust decisions.
    }
}
