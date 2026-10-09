# Engineering Feature Delivery SOP

## Steps

1. **Implement Feature and Open Pull Request** — Implement the feature according to specification, run unit tests, and open a pull request.
   - do: Follow code style guidelines and write unit tests
   - do: Run lint and type checking before committing
   - dont: Do not push untested code directly to master
   - dont: Do not bypass security boundaries

2. **Code Review and Architecture Verification** — Perform peer code review and verify architectural compliance.
   - do: Verify test coverage and edge cases
   - do: Check for separation of duties and single source of truth
   - dont: Do not approve without inspecting test results

3. **Integration and QA Validation** — Run end-to-end integration tests and verify regression safety.
   - do: Test happy path and error handling flows
   - dont: Do not skip flaky test analysis
