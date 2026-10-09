//! Model provider pricing structures and resolution from configuration.

use std::collections::HashMap;
use crate::schema::Config;

pub type ModelProviderPricing = HashMap<String, HashMap<String, f64>>;

pub fn build_model_provider_pricing(config: &Config) -> ModelProviderPricing {
    let mut pricing: ModelProviderPricing = HashMap::new();

    for (type_k, alias_k, profile) in config.providers.models.iter_entries() {
        let mut slot = profile.pricing.clone();
        apply_rate_sheet_pricing(config, type_k, &mut slot);
        if !slot.is_empty() {
            pricing.insert(format!("{type_k}.{alias_k}"), slot);
        }
    }

    pricing
}

pub fn build_type_level_model_provider_pricing(config: &Config) -> ModelProviderPricing {
    let mut pricing: ModelProviderPricing = HashMap::new();

    for (type_k, _alias_k, profile) in config.providers.models.iter_entries() {
        if profile.pricing.is_empty() {
            continue;
        }
        let slot = pricing.entry(type_k.to_string()).or_default();
        merge_pricing(slot, &profile.pricing);
    }

    for (provider_type, _model_id, _rates) in config.cost.rates.providers.models.iter_entries() {
        let slot = pricing.entry(provider_type.to_string()).or_default();
        apply_rate_sheet_pricing(config, provider_type, slot);
    }

    pricing
}

pub fn provider_pricing<'a>(
    pricing: &'a ModelProviderPricing,
    model_provider_name: &str,
) -> Option<&'a HashMap<String, f64>> {
    if let Some(slot) = pricing.get(model_provider_name) {
        return Some(slot);
    }

    // Type-keyed maps are still used in the channels path; when the lookup
    // arrives as `<type>.<alias>`, fall back to the bare provider family.
    if let Some((provider_type, _alias)) = model_provider_name.split_once('.')
        && let Some(slot) = pricing.get(provider_type)
    {
        return Some(slot);
    }

    // Some call sites surface only the bare provider type while the
    // pricing view is keyed by `<type>.<alias>`. Fall back only when the
    // type resolves to exactly one alias entry so pricing stays deterministic.
    let prefix = format!("{model_provider_name}.");
    let mut matches = pricing
        .iter()
        .filter(|(key, _)| key.starts_with(&prefix))
        .map(|(_, slot)| slot);
    let first = matches.next()?;
    if matches.next().is_none() {
        Some(first)
    } else {
        None
    }
}

fn apply_rate_sheet_pricing(config: &Config, provider_type: &str, slot: &mut HashMap<String, f64>) {
    for (rate_provider_type, model_id, rates) in config.cost.rates.providers.models.iter_entries() {
        if rate_provider_type != provider_type {
            continue;
        }
        if let Some(input) = rates.input_per_mtok {
            slot.insert(format!("{model_id}.input"), input);
        }
        if let Some(output) = rates.output_per_mtok {
            slot.insert(format!("{model_id}.output"), output);
        }
        if let Some(cached) = rates.cached_input_per_mtok {
            slot.insert(format!("{model_id}.cached_input"), cached);
        }
        if let Some(write) = rates.cache_write_per_mtok {
            slot.insert(format!("{model_id}.cache_write"), write);
        }
    }
}

fn merge_pricing(slot: &mut HashMap<String, f64>, pricing: &HashMap<String, f64>) {
    for (key, value) in pricing {
        slot.insert(key.clone(), *value);
    }
}
