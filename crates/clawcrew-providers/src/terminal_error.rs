//! User-facing rendering for provider terminal errors (PR-32 / BR-C5).
//!
//! Extracted from `clawcrew_runtime::agent::turn::outcome` to `clawcrew-providers`
//! so that gateway and other consumers can format terminal completion errors
//! without depending on the runtime agent subsystem.

use crate::{
    ReliableProviderTerminalFailure, ReliableProviderTerminalFailureKind,
    ReliableSemanticEmptyCompletion, model_refusal_from_error,
};

pub fn semantic_empty_terminal_completion_message_with_renderer<F>(
    agent_name: Option<&str>,
    render: F,
) -> String
where
    F: Fn(&str, &[(&str, &str)]) -> String,
{
    match agent_name {
        Some(agent_name) => render(
            "cli-delegate-error-invalid-semantic-completion",
            &[("agent_name", agent_name)],
        ),
        None => render("cli-agent-error-invalid-semantic-completion", &[]),
    }
}

pub fn pre_executed_tools_without_final_response_message<F>(
    agent_name: Option<&str>,
    render: F,
) -> String
where
    F: Fn(&str, &[(&str, &str)]) -> String,
{
    match agent_name {
        Some(agent_name) => render(
            "cli-delegate-error-incomplete-after-provider-tools",
            &[("agent_name", agent_name)],
        ),
        None => render("cli-agent-error-incomplete-after-provider-tools", &[]),
    }
}

pub fn reliable_provider_terminal_failure_message_with_renderer<F>(
    failure: &ReliableProviderTerminalFailure,
    render: F,
) -> String
where
    F: Fn(&str, &[(&str, &str)]) -> String,
{
    match failure.kind() {
        ReliableProviderTerminalFailureKind::ContextWindow => {
            render("cli-agent-error-provider-context-window", &[])
        }
        ReliableProviderTerminalFailureKind::CredentialsMissing => match failure.provider() {
            Some(provider) => render(
                "cli-agent-error-provider-credentials-missing-named",
                &[("provider", provider)],
            ),
            None => render("cli-agent-error-provider-credentials-missing", &[]),
        },
        ReliableProviderTerminalFailureKind::Authentication => match failure.provider() {
            Some(provider) => render(
                "cli-agent-error-provider-authentication-named",
                &[("provider", provider)],
            ),
            None => render("cli-agent-error-provider-authentication", &[]),
        },
        ReliableProviderTerminalFailureKind::RateLimited => {
            render("cli-agent-error-provider-rate-limited", &[])
        }
        ReliableProviderTerminalFailureKind::ProviderServer => {
            render("cli-agent-error-provider-server", &[])
        }
        ReliableProviderTerminalFailureKind::ModelNotFound => {
            render("cli-agent-error-provider-model-not-found", &[])
        }
        ReliableProviderTerminalFailureKind::ClientRequest => {
            render("cli-agent-error-provider-client-request", &[])
        }
        ReliableProviderTerminalFailureKind::Connection => match failure.endpoint() {
            Some(endpoint) if failure.endpoint_is_local() => render(
                "cli-agent-error-provider-connection-local",
                &[("endpoint", endpoint)],
            ),
            Some(endpoint) => render(
                "cli-agent-error-provider-connection-remote",
                &[("endpoint", endpoint)],
            ),
            None => render("cli-agent-error-provider-connection", &[]),
        },
        ReliableProviderTerminalFailureKind::Timeout => {
            render("cli-agent-error-provider-timeout", &[])
        }
        ReliableProviderTerminalFailureKind::Other => {
            render("cli-agent-error-provider-generic", &[])
        }
    }
}

pub fn is_semantic_empty_terminal_completion(err: &anyhow::Error) -> bool {
    err.chain().any(|source| {
        source.is::<clawcrew_api::model_provider::SemanticEmptyTerminalCompletion>()
            || source.is::<ReliableSemanticEmptyCompletion>()
            || source
                .to_string()
                .contains("provider stream completed without final text or tool calls")
    })
}

pub fn terminal_completion_error_message_with_renderer<F>(
    err: &anyhow::Error,
    agent_name: Option<&str>,
    render: F,
) -> Option<String>
where
    F: Fn(&str, &[(&str, &str)]) -> String,
{
    if model_refusal_from_error(err).is_some() {
        return Some(render("cli-agent-error-provider-refusal", &[]));
    }
    if let Some(failure) = err
        .chain()
        .find_map(|source| source.downcast_ref::<ReliableProviderTerminalFailure>())
    {
        return Some(reliable_provider_terminal_failure_message_with_renderer(
            failure, &render,
        ));
    }
    if is_semantic_empty_terminal_completion(err) {
        return Some(semantic_empty_terminal_completion_message_with_renderer(
            agent_name, &render,
        ));
    }
    if err
        .to_string()
        .contains("provider stream ended after provider-executed tools without a final response")
    {
        return Some(pre_executed_tools_without_final_response_message(
            agent_name, &render,
        ));
    }
    None
}
