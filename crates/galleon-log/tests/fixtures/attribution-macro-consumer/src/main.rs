use clawcrew_log_attribution_macro_support::FixtureAttributable;

fn main() {
    let _span = galleon_log::attribution_span!(&FixtureAttributable);
}
