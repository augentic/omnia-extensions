## 0.35.0

Unreleased

### Added

### Changed

- ([#8](https://github.com/augentic/omnia-extensions/pull/8)) Task runner moved from cargo-make to [mise](https://mise.jdx.dev); `Makefile` forwards to `mise run`, workflows use versioned `augentic/toolkit` reusable jobs, and the separate `wasm32-wasip2` CI job folded into shared `lint`.
- ([#21](https://github.com/augentic/omnia-extensions/pull/21)) Adopted shared organisation conventions from `augentic/toolkit` v0.3.0 (`AGENTS.md`, licences, governance, `rustfmt.toml`, `taplo.toml`, and related managed files).
- ([#22](https://github.com/augentic/omnia-extensions/pull/22)) Bumped shared conventions to `augentic/toolkit` v0.4.0 (key-owned TOML tables, unified managed notice, dropped `conventions.toml`).
- ([#23](https://github.com/augentic/omnia-extensions/pull/23)) Refreshed conventions sync: CI reusable workflow pins and workspace Clippy lint table aligned with toolkit.
- ([#24](https://github.com/augentic/omnia-extensions/pull/24)) Replaced `cfg_if::cfg_if!` with std `cfg_select!` and raised `rust-version` to 1.99; dropped the `cfg-if` dependency.
- ([#25](https://github.com/augentic/omnia-extensions/pull/25)) Pinned `augentic/toolkit` workflows and mise `?ref=` to v0.5.1.
- ([#26](https://github.com/augentic/omnia-extensions/pull/26)) Pinned `augentic/toolkit` to v0.6.0 (Clippy `each-feature` only for crates with features; doc tests run in the main test job).
- ([#46](https://github.com/augentic/omnia-extensions/pull/46)) `cargo update` and refreshed `cargo-vet` supply-chain metadata (RustSec fixes for `rustls` and `wasmtime`).
- ([#47](https://github.com/augentic/omnia-extensions/pull/47)) Removed dormant `renovate.json` (Renovate is not enabled on the org).
- ([#48](https://github.com/augentic/omnia-extensions/pull/48)) Restored Dependabot ignore for `augentic/*` so toolkit pins are bumped by hand, not auto PRs.
- ([#57](https://github.com/augentic/omnia-extensions/pull/57)) Workspace omnia crates bumped to **0.37.0** (Wasmtime 50.0.0); `allowed-duplicate-crates` extended for transitive OTel / lock-api duplicates.

<!-- Release notes generated using configuration in .github/release.yaml at ab456108586b24e962d86ad13d00e87d6c117600 -->

## What's Changed
* Bump to 0.35.0 by @augentic-releases[bot] in https://github.com/augentic/omnia-extensions/pull/7
* cut over to mise and versioned shared workflows by @andrew-goldie in https://github.com/augentic/omnia-extensions/pull/8
* Adopt the shared conventions from augentic/toolkit v0.3.0 by @andrewweston in https://github.com/augentic/omnia-extensions/pull/21
* Bump the shared conventions to augentic/toolkit v0.4.0 by @andrewweston in https://github.com/augentic/omnia-extensions/pull/22
* Update conventions by @andrewweston in https://github.com/augentic/omnia-extensions/pull/23
* Adopt cfg_select! and bump MSRV to 1.99 by @andrewweston in https://github.com/augentic/omnia-extensions/pull/24
* Bump toolkit version by @andrewweston in https://github.com/augentic/omnia-extensions/pull/25
* Bump toolkit version by @andrewweston in https://github.com/augentic/omnia-extensions/pull/26
* Update dependencies by @andrewweston in https://github.com/augentic/omnia-extensions/pull/46
* Remove Renovate config by @andrewweston in https://github.com/augentic/omnia-extensions/pull/47
* Ignore augentic/toolkit in Dependabot by @andrewweston in https://github.com/augentic/omnia-extensions/pull/48
* Upgrade omnia to wasmtime 50.0.0 by @andrewweston in https://github.com/augentic/omnia-extensions/pull/57
* Draft 0.35.0 release notes by @andrewweston in https://github.com/augentic/omnia-extensions/pull/58

## New Contributors
* @augentic-releases[bot] made their first contribution in https://github.com/augentic/omnia-extensions/pull/7
* @andrewweston made their first contribution in https://github.com/augentic/omnia-extensions/pull/21

**Full Changelog**: https://github.com/augentic/omnia-extensions/compare/v0.34.0...v0.35.0

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
* [0.34.x](https://github.com/augentic/omnia-extensions/blob/release-0.34.0/RELEASES.md)
