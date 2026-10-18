# Self-hosted CI runners

Two Docker containers register to the repo as GitHub Actions runners:

| Service | Label | Runs | Limits |
|---|---|---|---|
| `heavy` | `galleon-heavy` | compile/test jobs (shared `CARGO_TARGET_DIR`, wiped above 60G) | 3 CPU, 14G |
| `light` | `galleon-light` | guards, labelers, auto-merge, Principle gates | 1 CPU, 4G |

Containers have no host home, no ssh keys, no Docker socket, all capabilities dropped.
Outbound connections only; no inbound port.

```bash
# first start (token is single-use and expires in 1h)
RUNNER_TOKEN=$(gh api -X POST repos/diezy-realm/galleon-fleet/actions/runners/registration-token --jq .token) \
  docker compose -f dev/ci-runner/docker-compose.yml up -d --build

docker compose -f dev/ci-runner/docker-compose.yml logs -f heavy   # watch a job
docker compose -f dev/ci-runner/docker-compose.yml down            # stop (state volumes stay)
```

Runner state lives in named volumes, so later restarts need no token. Toolchain versions come from
`rust-toolchain.toml`; tool versions (nextest, cargo-deny, cargo-audit, runner) are pinned with
checksums in `Dockerfile`.

Not covered: Windows/macOS/Nix/Docker-daemon lanes, MSRV, 32-bit, benches. Run those manually
(`workflow_dispatch`) or locally via `dev/ci.sh`.

Security note: a runner executes PR code. Keep the repo private and do not enable fork PR workflows.
