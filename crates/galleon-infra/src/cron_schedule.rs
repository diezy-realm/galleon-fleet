use crate::cron_types::Schedule;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use cron::Schedule as CronExprSchedule;
use std::str::FromStr;

pub fn next_run_for_schedule(schedule: &Schedule, from: DateTime<Utc>) -> Result<DateTime<Utc>> {
    match schedule {
        Schedule::Cron { expr, tz } => {
            let normalized = normalize_expression(expr)?;
            let cron = CronExprSchedule::from_str(&normalized)
                .with_context(|| format!("Invalid cron expression: {expr}"))?;

            if let Some(tz_name) = tz {
                let timezone = chrono_tz::Tz::from_str(tz_name)
                    .with_context(|| format!("Invalid IANA timezone: {tz_name}"))?;
                let localized_from = from.with_timezone(&timezone);
                let next_local = cron.after(&localized_from).next().ok_or_else(|| {
                    ::clawcrew_log::record!(
                        WARN,
                        ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Fail)
                            .with_outcome(::clawcrew_log::EventOutcome::Failure)
                            .with_attrs(::serde_json::json!({"expr": expr})),
                        "cron schedule: no future occurrence for expression"
                    );
                    anyhow::Error::msg(format!("No future occurrence for expression: {expr}"))
                })?;
                Ok(next_local.with_timezone(&Utc))
            } else {
                // Default to OS local timezone so schedules match user
                // expectations instead of always using UTC.
                let local_from = from.with_timezone(&chrono::Local);
                let next_local = cron.after(&local_from).next().ok_or_else(|| {
                    ::clawcrew_log::record!(
                        WARN,
                        ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Fail)
                            .with_outcome(::clawcrew_log::EventOutcome::Failure)
                            .with_attrs(::serde_json::json!({"expr": expr})),
                        "cron schedule: no future occurrence for expression"
                    );
                    anyhow::Error::msg(format!("No future occurrence for expression: {expr}"))
                })?;
                Ok(next_local.with_timezone(&Utc))
            }
        }
        Schedule::At { at } => Ok(*at),
        Schedule::Every { every_ms } => {
            if *every_ms == 0 {
                anyhow::bail!("Invalid schedule: every_ms must be > 0");
            }
            let ms = i64::try_from(*every_ms).context("every_ms is too large")?;
            let delta = ChronoDuration::milliseconds(ms);
            from.checked_add_signed(delta).ok_or_else(|| {
                ::clawcrew_log::record!(
                    ERROR,
                    ::clawcrew_log::Event::new(module_path!(), ::clawcrew_log::Action::Fail)
                        .with_outcome(::clawcrew_log::EventOutcome::Failure)
                        .with_attrs(::serde_json::json!({"every_ms": *every_ms})),
                    "cron schedule: every_ms overflowed DateTime arithmetic"
                );
                anyhow::Error::msg("every_ms overflowed DateTime")
            })
        }
    }
}

pub fn validate_schedule(schedule: &Schedule, now: DateTime<Utc>) -> Result<()> {
    match schedule {
        Schedule::Cron { expr, .. } => {
            let _ = normalize_expression(expr)?;
            let _ = next_run_for_schedule(schedule, now)?;
            Ok(())
        }
        Schedule::At { at } => {
            if *at <= now {
                anyhow::bail!(
                    "Invalid schedule: 'at' must be in the future \
                     (now_utc={}, now_local={}, at_utc={}, at_local={}, delta_seconds={})",
                    now.to_rfc3339(),
                    now.with_timezone(&chrono::Local).to_rfc3339(),
                    at.to_rfc3339(),
                    at.with_timezone(&chrono::Local).to_rfc3339(),
                    (*at - now).num_seconds()
                );
            }
            Ok(())
        }
        Schedule::Every { every_ms } => {
            if *every_ms == 0 {
                anyhow::bail!("Invalid schedule: every_ms must be > 0");
            }
            Ok(())
        }
    }
}

pub fn schedule_cron_expression(schedule: &Schedule) -> Option<String> {
    match schedule {
        Schedule::Cron { expr, .. } => Some(expr.clone()),
        _ => None,
    }
}

pub fn normalize_expression(expression: &str) -> Result<String> {
    let expression = expression.trim();
    let field_count = expression.split_whitespace().count();

    match field_count {
        5 => {
            let mut fields: Vec<&str> = expression.split_whitespace().collect();
            let weekday = fields[4];
            let normalized_weekday = normalize_weekday_field(weekday)?;
            fields[4] = &normalized_weekday;
            Ok(format!(
                "0 {} {} {} {} {}",
                fields[0], fields[1], fields[2], fields[3], fields[4]
            ))
        }
        6 | 7 => Ok(expression.to_string()),
        _ => anyhow::bail!(
            "Invalid cron expression: {expression} (expected 5, 6, or 7 fields, got {field_count})"
        ),
    }
}

fn translate_weekday_value(val: u8) -> Result<u8> {
    match val {
        0 | 7 => Ok(1), // Sunday
        1..=6 => Ok(val + 1),
        _ => anyhow::bail!("Invalid weekday value: {val} (expected 0-7)"),
    }
}

fn standard_weekday_range_values(start: u8, end: u8) -> Vec<u8> {
    let mut standard_days: Vec<u8> = Vec::new();
    let mut day = start;
    loop {
        standard_days.push(day);
        if day == end {
            break;
        }
        day = if day >= 7 { 0 } else { day + 1 };
    }
    standard_days
}

fn translated_weekday_list(mut standard_days: Vec<u8>) -> Result<String> {
    let mut translated: Vec<u8> = standard_days
        .drain(..)
        .map(translate_weekday_value)
        .collect::<Result<Vec<u8>>>()?;
    translated.sort_unstable();
    translated.dedup();

    Ok(translated
        .into_iter()
        .map(|v| v.to_string())
        .collect::<Vec<_>>()
        .join(","))
}

fn expand_weekday_range(start: u8, end: u8) -> Result<String> {
    translated_weekday_list(standard_weekday_range_values(start, end))
}

fn expand_stepped_weekday_range(start: u8, end: u8, step: u8) -> Result<String> {
    if step == 0 {
        anyhow::bail!("Invalid weekday step: 0");
    }

    let standard_days = standard_weekday_range_values(start, end)
        .into_iter()
        .step_by(step as usize)
        .collect();

    translated_weekday_list(standard_days)
}

fn normalize_weekday_field(field: &str) -> Result<String> {
    if field == "*" || field == "?" {
        return Ok(field.to_string());
    }

    if field.chars().any(|c| c.is_ascii_alphabetic()) {
        return Ok(field.to_string());
    }

    let parts: Vec<&str> = field.split(',').collect();
    let mut result_parts = Vec::with_capacity(parts.len());

    for part in parts {
        let (range_part, step) = if let Some((r, s)) = part.split_once('/') {
            (r, Some(s))
        } else {
            (part, None)
        };

        let (translated, expanded_step_applied) =
            if let Some((start_s, end_s)) = range_part.split_once('-') {
                let start: u8 = start_s
                    .parse()
                    .with_context(|| format!("Invalid weekday in range: {start_s}"))?;
                let end: u8 = end_s
                    .parse()
                    .with_context(|| format!("Invalid weekday in range: {end_s}"))?;
                let new_start = translate_weekday_value(start)?;
                let new_end = translate_weekday_value(end)?;
                if new_start < new_end || (new_start == new_end && start == end) {
                    (format!("{new_start}-{new_end}"), false)
                } else {
                    if let Some(s) = step {
                        let step: u8 = s
                            .parse()
                            .with_context(|| format!("Invalid weekday step: {s}"))?;
                        (expand_stepped_weekday_range(start, end, step)?, true)
                    } else {
                        (expand_weekday_range(start, end)?, false)
                    }
                }
            } else if range_part == "*" {
                ("*".to_string(), false)
            } else {
                let val: u8 = range_part
                    .parse()
                    .with_context(|| format!("Invalid weekday value: {range_part}"))?;
                (translate_weekday_value(val)?.to_string(), false)
            };

        if let Some(s) = step.filter(|_| !expanded_step_applied) {
            result_parts.push(format!("{translated}/{s}"));
        } else {
            result_parts.push(translated);
        }
    }

    Ok(result_parts.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn next_run_for_schedule_supports_every_and_at() {
        let now = Utc::now();
        let every = Schedule::Every { every_ms: 60_000 };
        let next = next_run_for_schedule(&every, now).unwrap();
        assert!(next > now);

        let at = now + ChronoDuration::minutes(10);
        let at_schedule = Schedule::At { at };
        let next_at = next_run_for_schedule(&at_schedule, now).unwrap();
        assert_eq!(next_at, at);
    }

    #[test]
    fn next_run_for_schedule_supports_timezone() {
        let from = Utc.with_ymd_and_hms(2026, 2, 16, 0, 0, 0).unwrap();
        let schedule = Schedule::Cron {
            expr: "0 9 * * *".into(),
            tz: Some("America/Los_Angeles".into()),
        };

        let next = next_run_for_schedule(&schedule, from).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 2, 16, 17, 0, 0).unwrap());
    }

    #[test]
    fn normalize_weekday_field_translates_standard_crontab_values() {
        assert_eq!(normalize_weekday_field("0").unwrap(), "1");
        assert_eq!(normalize_weekday_field("1").unwrap(), "2");
        assert_eq!(normalize_weekday_field("5").unwrap(), "6");
        assert_eq!(normalize_weekday_field("6").unwrap(), "7");
        assert_eq!(normalize_weekday_field("7").unwrap(), "1");
    }

    #[test]
    fn normalize_weekday_field_translates_ranges() {
        assert_eq!(normalize_weekday_field("1-5").unwrap(), "2-6");
        assert_eq!(normalize_weekday_field("0-6").unwrap(), "1-7");
    }
}
