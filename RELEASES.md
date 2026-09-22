## 0.34.0

Released 2026-09-22

### Added

- `omnia-http-cache`: `HttpCache<H, S>` decorator over `HttpRequest` + `StateStore`
  answering `Cache-Control` / `If-None-Match` requests from an injected store. First release.
- `omnia-orm`: `entity!` mapping and typed `SELECT` / `INSERT` / `UPDATE` / `DELETE`
  builders over `wasi:sql`, moved here from the omnia repository (last published from
  there as 0.33.0). Builds against omnia 0.36.0.

### Changed

<!-- Release notes generated using configuration in .github/release.yaml at main -->

## What's Changed
* standard github workflows by @andrew-goldie in https://github.com/augentic/omnia-extensions/pull/1
* Cache by @andrew-goldie in https://github.com/augentic/omnia-extensions/pull/2
* ORM extension moved from Omnia by @andrew-goldie in https://github.com/augentic/omnia-extensions/pull/3
* fix lint target configuration by @andrew-goldie in https://github.com/augentic/omnia-extensions/pull/4
* remove patches and use public omnia* v0.36.0 by @andrew-goldie in https://github.com/augentic/omnia-extensions/pull/5
* Prepare 0.34.0 release by @andrew-goldie in https://github.com/augentic/omnia-extensions/pull/6

## New Contributors
* @andrew-goldie made their first contribution in https://github.com/augentic/omnia-extensions/pull/1

**Full Changelog**: https://github.com/augentic/omnia-extensions/commits/v0.34.0

---

Release notes for previous releases can be found on the respective release branches of the repository.

<!-- ARCHIVE_START -->
