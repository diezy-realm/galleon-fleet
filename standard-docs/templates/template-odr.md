# ODR-YYYY-MM-DD: [Owner decision]

- **Status:** Proposed | Ratified | Superseded
- **Date:** YYYY-MM-DD
- **Prepared by:** [Role or team]
- **Ratifying authority:** [Owner or explicitly delegated authority]
- **Approval evidence:** [Reference, date and exact scope]
- **Effective date:** YYYY-MM-DD
- **Related:** ODR/ADR/requirement identifiers
- **Supersedes / amends:** Exact records and clauses, or none

## 1. Context and problem

State the strategic problem and verified evidence. Label assumptions explicitly.

## 2. Decision

List binding decisions and limits. Preparation by an agent does not grant
ratification authority. Routine implementation choices do not require new ODRs.

## 3. Guardrails and delegation

State forbidden actions, authorized actors, approval boundaries and escalation.
For external execution, define permitted data, providers, budget and authority.
A reviewer verdict does not independently authorize merge or a new product mandate.

## 4. Architecture and operational impact

- Tier 1: Tauri desktop shell.
- Tier 2: Rust Core, workspace crates and authenticated gateway.
- Tier 3: UI Web.
- External add-ons: separate repositories through public HTTP/WS, MCP or WASM.
- Operational tooling: identify the owning repository; write N/A for unaffected tiers.

## 5. Implementation and verification

Link implementation work and observable invariants. Include relevant test evidence,
migration, limits and rollback. Separate intended behavior from shipped behavior.

## 6. Lifecycle

Ratified ODRs are immutable. Changes require a new authorized record with explicit
supersession/amendment clauses and an update to the existing supersession index.
Do not edit historical mandates to make them appear newly approved.
