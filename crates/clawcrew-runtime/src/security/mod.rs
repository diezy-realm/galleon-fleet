//! Security subsystem for policy enforcement, sandboxing, and secret management.

pub mod appcontainer;
pub mod audit;
pub mod auth_provider;
#[cfg(feature = "sandbox-bubblewrap")]
pub mod bubblewrap;
pub mod cert_ledger;
pub mod detect;
pub mod docker;

// Prompt injection defense (contributed from RustyClaw, MIT licensed)
pub mod domain_matcher;
pub mod estop;
pub mod external_content;
#[cfg(target_os = "linux")]
pub mod firejail;
pub mod iam_policy;
pub mod ingress;
#[cfg(feature = "sandbox-landlock")]
pub mod landlock;
pub mod leak_detector;
pub mod nevis;
pub mod otp;
pub mod pairing;
pub mod playbook;
pub mod policy;
pub mod principal_resolver;
pub mod prompt_guard;
#[cfg(target_os = "macos")]
pub mod seatbelt;
pub mod secrets;
pub mod traits;
pub mod vulnerability;
#[cfg(feature = "webauthn")]
pub mod webauthn;

#[allow(unused_imports)]
pub use appcontainer::AppContainerSandbox;
pub use audit::{AuditEvent, AuditEventType, AuditLogger};
#[allow(unused_imports)]
pub use detect::create_sandbox;
pub use detect::linux_memcg_available;
pub use detect::{SandboxExtraRoots, SandboxPosture, sandbox_posture};
pub use domain_matcher::DomainMatcher;
#[allow(unused_imports)]
pub use estop::{EstopLevel, EstopManager, EstopState, ResumeSelector};
#[allow(unused_imports)]
pub use external_content::{
    ContentSafety, FramingPolicy, OutboundPolicy, ScanOutcome, ScanPolicy, ScreenVerdict,
    cap_untrusted, frame_untrusted, new_marker_id, sanitize_untrusted, scan_untrusted,
    scrub_outbound,
};
// Universal ingress policy front door.
#[allow(unused_imports)]
pub use ingress::{IngressPolicy, ingress_policy};
#[allow(unused_imports)]
pub use otp::OtpValidator;
#[allow(unused_imports)]
pub use pairing::PairingGuard;
pub use policy::{AutonomyLevel, SecurityPolicy};
#[allow(unused_imports)]
pub use secrets::SecretStore;
#[allow(unused_imports)]
pub use traits::{NoopSandbox, Sandbox};
// Nevis IAM integration
#[allow(unused_imports)]
pub use iam_policy::{IamPolicy, PolicyDecision};
#[allow(unused_imports)]
pub use nevis::{NevisAuthProvider, NevisIdentity};
// Prompt injection defense exports
#[allow(unused_imports)]
pub use leak_detector::{LeakDetector, LeakResult};
#[allow(unused_imports)]
pub use prompt_guard::{GuardAction, GuardResult, PromptGuard};
pub use galleon_security_scrub::{redact, scrub, scrub_with_config};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reexported_policy_and_pairing_types_are_usable() {
        let policy = SecurityPolicy::default();
        assert_eq!(policy.autonomy, AutonomyLevel::Supervised);

        let guard = PairingGuard::new(false, &[], pairing::PairingCodePolicy::default());
        assert!(!guard.require_pairing());
    }

    #[test]
    fn reexported_secret_store_encrypt_decrypt_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        let store = SecretStore::new(temp.path(), false);

        let encrypted = store.encrypt("top-secret").unwrap();
        let decrypted = store.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, "top-secret");
    }

    #[test]
    fn redact_hides_most_of_value() {
        assert_eq!(redact("abcdefgh"), "abcd***");
        assert_eq!(redact("ab"), "***");
        assert_eq!(redact(""), "***");
        assert_eq!(redact("12345"), "1234***");
    }

    #[test]
    fn redact_handles_multibyte_utf8_without_panic() {
        // CJK characters are 3 bytes each; slicing at byte 4 would panic
        // without char-boundary-safe handling.
        let result = redact("密码是很长的秘密");
        assert!(result.ends_with("***"));
        assert!(result.is_char_boundary(result.len()));
    }
}
