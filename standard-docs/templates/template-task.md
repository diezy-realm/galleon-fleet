# Task reference: [Stable identifier and title]

- **Canonical source:** [Board TASK-NNN or owning roadmap ticket]
- **Priority:** P0 | P1 | P2 | P3
- **Requirement/specification:** [Identifier and revision]
- **PR and target:** [Reference; board delivery targets testing]
- **Dependencies:** [Required outputs and rationale]
- **Conflicts:** [Write scope and exclusive resources]

This document is a planning/reference view. Board execution uses the canonical
JSON task defined by board TEMPLATES.md. Do not create a second status, run ledger
or completion checklist here. Existing roadmap tickets retain their current owners.

## 1. Problem and evidence

Observed code/commit, affected users, expected outcome and explicit exclusions.

## 2. Change and verification plan

Map each acceptance criterion to allowed files, commands and expected evidence.
Include the tests needed to prove behavior in the write scope.
Do not add ignored tests or weaken a gate to produce a successful delivery.

## 3. Execution requirements

State required capabilities, permissions and execution location when relevant.
The controller selects a configured executor and owns identity/run metadata.
No implicit nested delegation or provider switch within an attempt.

## 4. Acceptance and evidence

Behavioral criteria, actual outputs, coverage gaps and source revisions.
Distinguish failed, skipped, advisory and baseline-waived checks from passing tests.
Delivery can remain partial pending acceptance and integration.

## 5. Completion

Link the canonical acceptance verdict, exact PR head/base and integration evidence.
Update only documentation owned by this task. An open or closed-unmerged PR is
not completion; board code tasks become done after verified merge.
