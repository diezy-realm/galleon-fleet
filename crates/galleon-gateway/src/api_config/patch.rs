//! JSON Patch operations and atomic config swap.

use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use galleon_config::api_error::{ConfigApiCode, ConfigApiError};
use galleon_config::typed_value::coerce_for_set_prop as json_to_setprop_string;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::{
    AppState, ConfigWriteGuard, DriftEntry, compute_drift, error_response, lookup_prop_field,
    map_prop_error, require_auth, scoped_validate,
};

/// One JSON Patch operation. Supports `add`, `remove`, `replace`, `test`, and
/// ClawCrew's `comment` extension. Every operation requires `path`; `add`,
/// `replace`, and `test` require `value`, while `comment` requires `comment`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct PatchOp {
    pub op: String,
    pub path: String,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
    #[serde(default)]
    pub comment: Option<String>,
}

/// Single result entry in a successful PATCH response, one per applied op.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct PatchOpResult {
    pub op: String,
    pub path: String,
    /// The resulting value at the target path after the op applied.
    /// `None` for secret paths (per the secrets-handling boundary), and for
    /// `remove` ops where the field was reset to its default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub populated: Option<bool>,
    /// Comment that was applied alongside this op (if any). Echoed so
    /// clients can confirm the comment was actually written to disk
    /// without having to round-trip through `GET` and parse the TOML.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "schema-export", derive(schemars::JsonSchema))]
pub struct PatchResponse {
    pub saved: bool,
    pub results: Vec<PatchOpResult>,
    /// Non-fatal validation warnings against the saved configuration.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<galleon_config::validation_warnings::ValidationWarning>,
}

pub(crate) fn parse_patch_ops(value: serde_json::Value) -> Result<Vec<PatchOp>, ConfigApiError> {
    let ops = value.as_array().ok_or_else(|| {
        ConfigApiError::new(
            ConfigApiCode::ValueTypeMismatch,
            "JSON Patch body must be a JSON array of operations",
        )
    })?;

    let mut parsed = Vec::with_capacity(ops.len());
    for (idx, op) in ops.iter().enumerate() {
        let object = op.as_object().ok_or_else(|| {
            ConfigApiError::new(
                ConfigApiCode::ValueTypeMismatch,
                format!("JSON Patch op[{idx}] must be an object"),
            )
            .with_op_index(idx)
        })?;
        let op_name = object.get("op").and_then(|v| v.as_str()).ok_or_else(|| {
            ConfigApiError::new(
                ConfigApiCode::ValueTypeMismatch,
                format!("JSON Patch op[{idx}] requires string `op` field"),
            )
            .with_op_index(idx)
        })?;
        let path = object.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
            ConfigApiError::new(
                ConfigApiCode::ValueTypeMismatch,
                format!("JSON Patch op[{idx}] requires string `path` field"),
            )
            .with_op_index(idx)
        })?;
        let comment = match object.get("comment") {
            Some(value) => Some(
                value
                    .as_str()
                    .ok_or_else(|| {
                        ConfigApiError::new(
                            ConfigApiCode::ValueTypeMismatch,
                            format!("JSON Patch op[{idx}] `comment` field must be a string"),
                        )
                        .with_path(json_pointer_to_dotted(path))
                        .with_op_index(idx)
                    })?
                    .to_string(),
            ),
            None => None,
        };

        parsed.push(PatchOp {
            op: op_name.to_string(),
            path: path.to_string(),
            value: object.get("value").cloned(),
            comment,
        });
    }

    Ok(parsed)
}

/// serialize the whole read-mutate-swap critical section on
/// `state.config_write_lock`, acquired before the caller's read-for-modify.
/// This function deliberately does NOT lock internally: the caller already
/// holds the guard, so re-locking here would deadlock. The `debug_assert!`
/// below catches a caller that passed a look-alike guard from the wrong
/// mutex instead of the one actually held.
pub async fn persist_and_swap(
    state: &AppState,
    mut new_config: galleon_config::schema::Config,
    _guard: &ConfigWriteGuard,
) -> Result<(), ConfigApiError> {
    debug_assert!(
        state.config_write_lock.try_lock().is_err(),
        "persist_and_swap caller must hold state.config_write_lock"
    );
    let config_path = new_config.config_path.clone();

    // Snapshot pre-write disk state (used for revert on save failure). Only
    // NotFound means the file was absent. Any other read failure must stop
    // before the save because treating an unreadable file as absent would
    // let the rollback path delete an existing canonical config.
    let snapshot = read_config_snapshot(&config_path).await?;

    if let Err(e) = new_config.save_dirty().await {
        if let Some(prev) = snapshot {
            let _ = tokio::fs::write(&config_path, prev).await;
        }
        // When the path was absent, the atomic writer either leaves it absent
        // on failure or reports a visible rename as success. Do not remove a
        // path here: an external writer may have created it after admission.
        return Err(ConfigApiError::new(
            ConfigApiCode::ReloadFailed,
            format!("save failed: {e}"),
        ));
    }

    *state.config.write() = new_config;
    state
        .pending_reload
        .store(true, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

async fn read_config_snapshot(
    config_path: &std::path::Path,
) -> Result<Option<Vec<u8>>, ConfigApiError> {
    match tokio::fs::read(config_path).await {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ConfigApiError::new(
            ConfigApiCode::ReloadFailed,
            format!("failed to snapshot existing config before save: {error}"),
        )),
    }
}

/// Reject masked or empty values from writes to secret-bearing properties.
/// Dashboard surfaces may send the masked display sentinel when no real edit
/// was made; accepting it would replace the live secret with that sentinel.
pub(crate) fn reject_masked_secret_value(
    path: &str,
    is_sensitive: bool,
    value: &str,
) -> Result<(), ConfigApiError> {
    if is_sensitive
        && (value == galleon_config::traits::MASKED_SECRET || value == "****" || value.is_empty())
    {
        return Err(ConfigApiError::new(
            ConfigApiCode::ValidationFailed,
            format!("Refusing to overwrite secret `{path}` with a masked or empty value"),
        )
        .with_path(path));
    }
    Ok(())
}

pub async fn handle_patch(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::Json(body): axum::Json<serde_json::Value>,
) -> Response {
    if let Err(e) = require_auth(&state, &headers) {
        return e.into_response();
    }

    let ops = match parse_patch_ops(body) {
        Ok(ops) => ops,
        Err(e) => return error_response(e),
    };

    let _cfg_guard = Arc::clone(&state.config_write_lock).lock_owned().await;
    let working = state.config.read().clone();

    let override_drift = headers
        .get("x-clawcrew-override-drift")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !override_drift {
        let drifted = compute_drift(&working).await;
        if !drifted.is_empty() {
            let touched: std::collections::HashSet<String> = ops
                .iter()
                .map(|op| json_pointer_to_dotted(&op.path))
                .collect();
            let conflicts: Vec<&DriftEntry> = drifted
                .iter()
                .filter(|d| touched.contains(&d.path))
                .collect();
            if !conflicts.is_empty() {
                let conflict_paths: Vec<String> =
                    conflicts.iter().map(|d| d.path.clone()).collect();
                return error_response(ConfigApiError::new(
                    ConfigApiCode::ConfigChangedExternally,
                    format!(
                        "on-disk config has drifted from in-memory state on \
                         {} path(s) being patched: {}. Send `X-ClawCrew-Override-Drift: true` \
                         to overwrite, or GET /api/config/drift to inspect first.",
                        conflicts.len(),
                        conflict_paths.join(", "),
                    ),
                ));
            }
        }
    }

    let mut working = working;
    let mut results = Vec::with_capacity(ops.len());

    for (idx, op) in ops.iter().enumerate() {
        let path = json_pointer_to_dotted(&op.path);
        if matches!(op.op.as_str(), "add" | "replace") && working.ensure_map_key_for_path(&path) {
            // Refused to vivify the reserved `default` agent: surface the same
            // reserved error the explicit create surfaces do, not a generic 404.
            return error_response(
                ConfigApiError::new(
                    ConfigApiCode::ValidationFailed,
                    "alias `default` is reserved and cannot be created",
                )
                .with_path(&path)
                .with_op_index(idx),
            );
        }
        let info = lookup_prop_field(&working, &path);
        let is_sensitive = info
            .as_ref()
            .map(|i| i.is_secret || i.derived_from_secret)
            .unwrap_or(false);

        match op.op.as_str() {
            "test" => {
                // Secret values can't leave the server, so a differential
                // test response would be the only signal — ban the op.
                if is_sensitive {
                    return error_response(
                        ConfigApiError::secret_test_forbidden(&path).with_op_index(idx),
                    );
                }
                let want = match op.value.as_ref() {
                    Some(v) => v.clone(),
                    None => {
                        return error_response(
                            ConfigApiError::new(
                                ConfigApiCode::ValueTypeMismatch,
                                "JSON Patch `test` op requires `value` field",
                            )
                            .with_path(&path)
                            .with_op_index(idx),
                        );
                    }
                };
                let actual_str = match working.get_prop(&path) {
                    Ok(v) => v,
                    Err(e) => return error_response(map_prop_error(e, &path).with_op_index(idx)),
                };
                let want_str = match json_to_setprop_string(&want, info.as_ref().map(|i| i.kind)) {
                    Ok(s) => s,
                    Err(e) => return error_response(e.with_path(&path).with_op_index(idx)),
                };
                if actual_str != want_str {
                    return error_response(
                        ConfigApiError::new(
                            ConfigApiCode::ValidationFailed,
                            format!("`test` op failed: expected {want_str:?}, got {actual_str:?}"),
                        )
                        .with_path(&path)
                        .with_op_index(idx),
                    );
                }
                results.push(PatchOpResult {
                    op: op.op.clone(),
                    path,
                    value: Some(serde_json::Value::String(actual_str)),
                    populated: None,
                    comment: None, // `test` ops don't write
                });
            }
            "add" | "replace" => {
                let value = match op.value.as_ref() {
                    Some(v) => v.clone(),
                    None => {
                        return error_response(
                            ConfigApiError::new(
                                ConfigApiCode::ValueTypeMismatch,
                                format!("JSON Patch `{}` op requires `value` field", op.op),
                            )
                            .with_path(&path)
                            .with_op_index(idx),
                        );
                    }
                };
                let value_str = match json_to_setprop_string(&value, info.as_ref().map(|i| i.kind))
                {
                    Ok(s) => s,
                    Err(e) => {
                        return error_response(e.with_path(&path).with_op_index(idx));
                    }
                };
                if let Err(e) = reject_masked_secret_value(&path, is_sensitive, &value_str) {
                    return error_response(e.with_op_index(idx));
                }
                if let Err(e) = working.set_prop_persistent(&path, &value_str) {
                    return error_response(map_prop_error(e, &path).with_op_index(idx));
                }
                if is_sensitive {
                    results.push(PatchOpResult {
                        op: op.op.clone(),
                        path,
                        value: None,
                        populated: Some(!value_str.is_empty()),
                        comment: op.comment.clone(),
                    });
                } else {
                    results.push(PatchOpResult {
                        op: op.op.clone(),
                        path,
                        value: Some(serde_json::Value::String(value_str)),
                        populated: None,
                        comment: op.comment.clone(),
                    });
                }
            }
            "remove" => {
                if let Err(e) = working.set_prop_persistent(&path, "") {
                    return error_response(map_prop_error(e, &path).with_op_index(idx));
                }
                if is_sensitive {
                    results.push(PatchOpResult {
                        op: op.op.clone(),
                        path,
                        value: None,
                        populated: Some(false),
                        comment: op.comment.clone(),
                    });
                } else {
                    results.push(PatchOpResult {
                        op: op.op.clone(),
                        path,
                        value: Some(serde_json::Value::Null),
                        populated: None,
                        comment: op.comment.clone(),
                    });
                }
            }
            "comment" => {
                // Comment-only update: record the (path, comment) pair
                // for `apply_comments` after the patch commits, but
                // skip `set_prop` entirely. Lets the operator annotate
                // a secret without rotating its ciphertext.
                if info.is_none() {
                    return error_response(
                        ConfigApiError::path_not_found(&path).with_op_index(idx),
                    );
                }
                let Some(comment) = op.comment.clone() else {
                    return error_response(
                        ConfigApiError::new(
                            ConfigApiCode::ValueTypeMismatch,
                            "JSON Patch `comment` op requires `comment` field",
                        )
                        .with_path(&path)
                        .with_op_index(idx),
                    );
                };
                results.push(PatchOpResult {
                    op: op.op.clone(),
                    path,
                    value: None,
                    populated: None,
                    comment: Some(comment),
                });
            }
            "move" | "copy" => {
                return error_response(
                    ConfigApiError::op_not_supported(&op.op)
                        .with_path(&path)
                        .with_op_index(idx),
                );
            }
            other => {
                return error_response(
                    ConfigApiError::new(
                        ConfigApiCode::OpNotSupported,
                        format!("unknown JSON Patch operation `{other}`"),
                    )
                    .with_path(&path)
                    .with_op_index(idx),
                );
            }
        }
    }

    // Per-PATCH validation is scoped to the dirty paths. See
    // `scoped_validate` for the contract.
    let scoped_validation_warnings = match scoped_validate(&working) {
        Ok(ws) => ws,
        Err(err) => return error_response(err),
    };

    // Collect (path, comment) pairs from any op that supplied a non-None
    // comment. Applied after save() so the comment-preserving sync_table
    // pass doesn't strip them.
    let annotations: Vec<(String, String)> = ops
        .iter()
        .zip(results.iter())
        .filter_map(|(op, res)| op.comment.as_ref().map(|c| (res.path.clone(), c.clone())))
        .collect();

    let config_path = working.config_path.clone();
    // Collect non-fatal validation warnings against the post-save state
    // before working is moved into persist_and_swap. Same signal as
    // `galleon_log::record!` from `validate()`, surfaced structured so dashboard
    // callers see it.
    let mut warnings = working.collect_warnings();
    warnings.extend(scoped_validation_warnings);
    if let Err(e) = persist_and_swap(&state, working, &_cfg_guard).await {
        return error_response(e);
    }
    if !annotations.is_empty()
        && let Err(e) =
            galleon_config::comment_writer::apply_comments(&config_path, &annotations).await
    {
        // Comments are best-effort decoration; surface as a non-fatal warn.
        // The patch itself succeeded — return success but log the failure.
        ::galleon_log::record!(
            WARN,
            ::galleon_log::Event::new(module_path!(), ::galleon_log::Action::Note)
                .with_outcome(::galleon_log::EventOutcome::Unknown)
                .with_attrs(::serde_json::json!({"error": format!("{}", e)})),
            "failed to apply PATCH op comments to config.toml"
        );
    }

    axum::Json(PatchResponse {
        saved: true,
        results,
        warnings,
    })
    .into_response()
}

pub(crate) fn json_pointer_to_dotted(path: &str) -> String {
    if path.starts_with('/') {
        path.trim_start_matches('/').replace('/', ".")
    } else {
        path.to_string()
    }
}
