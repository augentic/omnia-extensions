## 0.35.0

Unreleased

### Added

### Changed

- Task runner moved from cargo-make to [mise](https://mise.jdx.dev): `mise.toml`
  includes the shared Rust tasks from `augentic/toolkit` v0.3.0 and keeps the
  example `build`/`run` tasks and the `wasm` release build locally. The
  `Makefile` forwards `make <task>` to `mise run <task>`. Workflows are
  pinned to `augentic/toolkit@v0.3.0`; the shared `lint` job now covers the
  `wasm32-wasip2` clippy pass, so the separate `wasm` CI job is gone.
  `renovate.json` bumps the toolkit pin as one pull request. The shared
  files — the workflow stubs, the lint and deny tables, the guest deny-list,
  `AGENTS.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `GOVERNANCE.md`, and
  the licences — are written by `make conventions-sync` from
  `conventions.toml` and held by `make conventions-check` in CI.

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
* [0.34.x](https://github.com/augentic/omnia-extensions/blob/release-0.34.0/RELEASES.md)
