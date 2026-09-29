## 0.35.0

Unreleased

### Added

### Changed

- Task runner moved from cargo-make to [mise](https://mise.jdx.dev): `mise.toml`
  includes the shared Rust tasks from `augentic/.github` v0.2.0 and keeps the
  example `build`/`run` tasks and the `wasm` release build locally. The
  `Makefile` forwards `make <task>` to `mise run <task>`. Workflows are
  pinned to `augentic/.github@v0.2.0`; the shared `lint` job now covers the
  `wasm32-wasip2` clippy pass, so the separate `wasm` CI job is gone.

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
* [0.34.x](https://github.com/augentic/omnia-extensions/blob/release-0.34.0/RELEASES.md)
