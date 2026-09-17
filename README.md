# Omnia Extensions

Guest-side libraries that compose on [omnia](https://github.com/augentic/omnia)
capability traits. They are not part of the runtime: omnia never depends on
this repository. Each crate is an opt-in decorator or helper that a guest
wraps around the SDK, and the same code runs inside a `wasm32-wasip2`
component and against `omnia_test` doubles natively.

## Relation to omnia

Dependency is strictly one-way. The omnia crates are resolved from GitHub
`main` via `[patch.crates-io]` in `Cargo.toml`; `Cargo.lock` records the
exact revision. Commented `path = "../omnia/crates/…"` lines in the same
patch block swap in a sibling checkout for local development — do not
commit them. Production backends live in
[omnia-backends](https://github.com/augentic/omnia-backends) and are a
host-side concern; nothing here talks to them.

## Crates

| Crate | Description |
| --- | --- |
| [`omnia-http-cache`](crates/http-cache) | `HttpCache<H, S>` decorator over `HttpRequest` + `StateStore`. Requests carrying `Cache-Control` and `If-None-Match` are answered from, and written back through, the injected store. |

## Quick start

Build the HTTP cache example guest and run it with the example host.
`UPSTREAM_URL` is read from the runtime environment via `wasi:config`; any
origin that answers `GET /resource` will do:

```shell
make build http-cache
UPSTREAM_URL=https://jsonplaceholder.cypress.io/posts/1 make run http-cache
```

`GET /cached` fetches `{UPSTREAM_URL}/resource` through `HttpCache`. The
first call reaches the origin and is stored for 300 seconds; the second is
served from the store. See [`examples/http-cache/README.md`](examples/http-cache/README.md)
and the crate [header contract](crates/http-cache/README.md).

## Testing

```shell
cargo make test              # cargo nextest run --locked --all --all-features
```

The suite is laid out as omnia's three rungs (see omnia's
[Testing Omnia-Based Code](https://github.com/augentic/omnia/blob/main/docs/guides/testing-omnia-code.md)
guide) plus a build gate for the example host. Everything runs under
`cargo make test`: nothing is `#[ignore]`d, and nothing needs installing
beyond `rust-toolchain.toml`, which carries the `wasm32-wasip2` target the
component rung compiles for.

### Handler rung

`crates/http-cache/tests/cache.rs` drives `HttpCache` natively against
`omnia_test::guest::Provider` (`MatchedHttp` + `Memory`). Scenarios cover
miss-then-hit, `no-cache` refresh (with `max-age`) and bypass (without),
`max-age=0` revalidation, `no-store` bypass, non-2xx and 206 not stored, a corrupt
entry and a store outage both degrading to the origin, pass-through without
`Cache-Control` or with only unrecognised directives, and malformed
directives. Unit tests
for the `Cache-Control` parser live beside it in `src/control.rs`.

### Component rung

`examples/tests/component.rs` boots the example guest — compiled to a
`wasm32-wasip2` component by `examples/build.rs` — through the example
host's own `runtime!` wiring over `omnia_test::host::Backends`. A loopback
origin records hits; one scenario, `cached_miss_then_hit`, asserts both
handler responses, a single origin fetch, and the keyvalue envelope under
the raw etag.

### Examples gate

`examples/tests/examples.rs::build` runs `cargo build --locked --examples`
from the package root. The host in `examples/http-cache/runtime.rs` is a
server that never exits, so it is compiled, not run; the component rung is
where its wiring is exercised.

## Development

```shell
make ci         # fmt, clippy (native + wasm), test, docs, vet, outdated, deny
```

`make ci` is the whole gate: every rung above, including the component rung
and the examples gate, runs inside `make test`. The workspace follows
omnia's conventions: stable toolchain (`rust-toolchain.toml`, with the
`wasm32-wasip2` target), edition 2024, workspace lints, `cargo vet`
supply-chain audits (`supply-chain/`), and CI as thin wrappers over the
reusable workflows in `augentic/.github`.

## License

MIT OR Apache-2.0
