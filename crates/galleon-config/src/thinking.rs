//! Thinking/Reasoning Level Control (PR-34 / R-13).
//!
//! Extracted from `galleon-runtime::agent::thinking` so channel crates and
//! config-driven surfaces can resolve reasoning parameters without linking
//! to the runtime monolith.

use crate::scattered_types::{
    MAX_BUDGET_TOKENS, MIN_BUDGET_TOKENS, NativeThinkingParams, ThinkingConfig, ThinkingLevel,
};

/// Parameters derived from a thinking level, applied to the LLM request.
#[derive(Debug, Clone, PartialEq)]
pub struct ThinkingParams {
    /// Temperature adjustment (added to the base temperature, clamped to 0.0..=2.0).
    pub temperature_adjustment: f64,
    /// Maximum tokens adjustment (added to any existing max_tokens setting).
    pub max_tokens_adjustment: i64,
    /// Optional system prompt prefix injected before the existing system prompt.
    pub system_prompt_prefix: Option<String>,
    /// Native extended thinking parameters, populated when the config enables
    /// native thinking and the level has a `budget_tokens` value.
    pub native_thinking: Option<NativeThinkingParams>,
}

pub fn parse_thinking_directive(message: &str) -> Option<(ThinkingLevel, String)> {
    let trimmed = message.trim_start();
    if !trimmed.starts_with("/think:") {
        return None;
    }

    // Extract the level token (everything between `/think:` and the next whitespace or end).
    let after_prefix = &trimmed["/think:".len()..];
    let level_end = after_prefix
        .find(|c: char| c.is_whitespace())
        .unwrap_or(after_prefix.len());
    let level_str = &after_prefix[..level_end];

    let level = ThinkingLevel::from_str_insensitive(level_str)?;

    let remaining = after_prefix[level_end..].trim_start().to_string();
    Some((level, remaining))
}

pub fn strip_thinking_directive(message: &str) -> std::borrow::Cow<'_, str> {
    match parse_thinking_directive(message) {
        Some((_, remaining)) => std::borrow::Cow::Owned(remaining),
        None => std::borrow::Cow::Borrowed(message),
    }
}

/// Convert a `ThinkingLevel` into concrete parameters for the LLM request.
pub fn apply_thinking_level(level: ThinkingLevel) -> ThinkingParams {
    match level {
        ThinkingLevel::Off => ThinkingParams {
            temperature_adjustment: -0.2,
            max_tokens_adjustment: -1000,
            system_prompt_prefix: Some(
                "Be extremely concise. Give direct answers without explanation \
                 unless explicitly asked. No preamble."
                    .into(),
            ),
            native_thinking: None,
        },
        ThinkingLevel::Minimal => ThinkingParams {
            temperature_adjustment: -0.1,
            max_tokens_adjustment: -500,
            system_prompt_prefix: Some(
                "Be concise and fast. Keep explanations brief. \
                 Prioritize speed over thoroughness."
                    .into(),
            ),
            native_thinking: None,
        },
        ThinkingLevel::Low => ThinkingParams {
            temperature_adjustment: -0.05,
            max_tokens_adjustment: 0,
            system_prompt_prefix: Some("Keep reasoning light. Explain only when helpful.".into()),
            native_thinking: None,
        },
        ThinkingLevel::Medium => ThinkingParams {
            temperature_adjustment: 0.0,
            max_tokens_adjustment: 0,
            system_prompt_prefix: None,
            native_thinking: None,
        },
        ThinkingLevel::High => ThinkingParams {
            temperature_adjustment: 0.05,
            max_tokens_adjustment: 1000,
            system_prompt_prefix: Some(
                "Think step by step. Provide thorough analysis and \
                 consider edge cases before answering."
                    .into(),
            ),
            native_thinking: None,
        },
        ThinkingLevel::Max => ThinkingParams {
            temperature_adjustment: 0.1,
            max_tokens_adjustment: 2000,
            system_prompt_prefix: Some(
                "Think very carefully and exhaustively. Break down the problem \
                 into sub-problems, consider all angles, verify your reasoning, \
                 and provide the most thorough analysis possible."
                    .into(),
            ),
            native_thinking: None,
        },
    }
}

/// Convert a `ThinkingLevel` into parameters, resolving native extended
/// thinking from the provided config.
pub fn apply_thinking_level_with_config(
    level: ThinkingLevel,
    config: &ThinkingConfig,
) -> ThinkingParams {
    let mut params = apply_thinking_level(level);
    if config.native_thinking
        && let Some(budget) = config.budget_tokens_for(level)
    {
        let clamped = budget.clamp(MIN_BUDGET_TOKENS, MAX_BUDGET_TOKENS);
        if clamped != budget {
            ::galleon_log::record!(
                WARN,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                    .with_category(::galleon_log::EventCategory::Agent)
                    .with_attrs(::serde_json::json!({
                        "requested": budget,
                        "clamped": clamped,
                        "min": MIN_BUDGET_TOKENS,
                        "max": MAX_BUDGET_TOKENS
                    })),
                "budget_tokens outside accepted range; clamping"
            );
        }
        params.native_thinking = Some(NativeThinkingParams {
            budget_tokens: clamped,
            display: config.display.to_display(),
        });
    }
    params
}

pub fn resolve_thinking_level(
    inline_directive: Option<ThinkingLevel>,
    session_override: Option<ThinkingLevel>,
    config: &ThinkingConfig,
) -> ThinkingLevel {
    inline_directive
        .or(session_override)
        .unwrap_or(config.default_level)
}

/// Clamp a temperature value to the valid range `[0.0, 2.0]`.
pub fn clamp_temperature(temp: f64) -> f64 {
    temp.clamp(0.0, 2.0)
}

pub struct ResolvedThinking {
    pub effective_message: String,
    pub params: ThinkingParams,
    pub effective_temperature: f64,
}

/// Validate thinking config at startup. Call once during agent
/// initialization to warn about unrecognized budget_tokens keys.
pub fn validate_thinking_config(config: &ThinkingConfig) {
    config.warn_unknown_budget_keys();
}

pub fn resolve_thinking_from_message(
    message: &str,
    config: &ThinkingConfig,
    base_temperature: f64,
) -> ResolvedThinking {
    let (directive, effective_message) = match parse_thinking_directive(message) {
        Some((level, remaining)) => {
            ::galleon_log::record!(
                INFO,
                ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                    .with_category(::galleon_log::EventCategory::Agent)
                    .with_attrs(::serde_json::json!({"thinking_level": format!("{level:?}")})),
                "Thinking directive parsed from message"
            );
            (Some(level), remaining)
        }
        None => (None, message.to_string()),
    };
    let level = resolve_thinking_level(directive, None, config);
    let params = apply_thinking_level_with_config(level, config);
    let effective_temperature = clamp_temperature(base_temperature + params.temperature_adjustment);
    ResolvedThinking {
        effective_message,
        params,
        effective_temperature,
    }
}
