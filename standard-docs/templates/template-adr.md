# ADR-XXXX: [Architecture decision]

- **Status:** Proposed | Accepted | Deprecated | Superseded
- **Date:** YYYY-MM-DD
- **Proposer:** [Role or team]
- **Decision owner:** [Authorized role and approval reference]
- **Scope:** Product architecture | Operational tooling
- **Related:** ODR/ADR identifiers and source requirements
- **Supersedes / amends:** Exact records and clauses, or none

## 1. Context and evidence

Describe the problem, observed code/commit, constraints and unresolved assumptions.
Product context: Desktop shell / Rust Core / UI Web, per ADR-0018.
Operational tooling belongs to its owning repository and does not relocate product logic.

## 2. Options

Compare viable options, including existing behavior. Record costs, limitations,
risks and evidence behind capability claims.

## 3. Decision

State the chosen option, rationale, scope and exclusions.
Distinguish a proposal from an authorized decision.

## 4. Ownership and boundaries

Name the canonical owner of state, policy, execution, review and integration.
Product inter-crate communication uses Rust function calls. Public clients use
the authenticated HTTP/WS gateway; Tauri IPC is limited to shell responsibilities.
Internal gRPC/protobuf is forbidden by the boundary policy.
Do not copy task status or operational credentials into product state.

## 5. Consequences and security

Record effective permissions, trust boundaries, secrets and data exported to services.
A model selection is distinct from a harness, identity, transport or execution host.

## 6. Concurrency and failure recovery

Where applicable: ownership leases, run identity, idempotency, cancellation,
timeouts, bounded retries, orphan jobs and stale result rejection.
Bind asynchronous evidence to the code and requirement revisions evaluated.
Explain N/A when there is no asynchronous execution.

## 7. Verification and rollout

Observable acceptance, exact checks, evidence, uncovered cases, migration,
shadow/canary activation criteria and rollback.
A green policy aggregate does not prove every underlying test passed.

## 8. Decision lifecycle

Record approval evidence and effective date. Preserve accepted records according
to repository policy. Use explicit supersession/amendment references rather
than silently replacing an accepted mandate.
