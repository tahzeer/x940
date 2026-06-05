# AGENTS.md: x940

## Build & test

```bash
cargo build                                 # whole workspace
cargo build -p x940rs                      # single crate
cargo check --workspace                     # fast compile-check

# Run tests (skip python crate: needs libpython3.12 to link)
cargo test -p x940rs -p x940                # all active tests
cargo test -p x940rs -- proptests           # property-based fuzz tests

# Run a single test
cargo test -p x940rs -- decoder_chain       # test module
cargo test -p x940rs -- parser              # test function
cargo test -p x940node                      # node binding unit tests

# CLI integration tests (binary-level, uses assert_cmd)
cargo test -p x940

# Lint & format
cargo clippy --workspace -- -D warnings
cargo fmt && cargo fmt --check
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc --no-deps --workspace

# Benchmarks
cargo bench -p x940rs

# Python binding (requires maturin + libpython3.12-dev)
uv pip install maturin
uv run maturin develop -m crates/python/Cargo.toml --release
uv run pytest crates/python/tests/ -v

# Node.js binding (requires npm)
cd crates/node && npm install && npm test

# WASM binding (requires wasm-pack + wasm32 target)
rustup target add wasm32-unknown-unknown
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
cargo check -p x940wasm --target wasm32-unknown-unknown
cd crates/wasm && npm test
```

`cargo test --workspace` will fail locally if Python dev headers are missing.
Use `-p x940rs -p x940` instead. Node.js `npm test` builds the napi addon
first then runs JS integration tests. WASM `npm test` builds with wasm-pack
then runs JS integration tests in Node.js.

## Project layout

```
crates/core/        ← all business logic (parser, models, decoders, serializers)
  src/decoders/     ← Tag86Decoder trait + 4 dialect implementations
  src/serializers/  ← json.rs, csv.rs, camt053.rs (shared helpers in mod.rs)
  tests/            ← integration tests (public API only)
  tests/data/       ← 6 .sta golden-file payloads
crates/cli/         ← thin adapter: clap -> core
crates/python/      ← thin adapter: PyO3 -> core (needs libpython3.12 to link;
                      no Rust unit tests possible: pytest only)
  tests/            ← pytest integration tests (run via `pytest crates/python/tests/`)
crates/node/        ← thin adapter: napi-rs -> core (cdylib, no Rust int. tests)
  tests/            ← JS integration tests only; Rust unit tests inline (8)
crates/wasm/        ← thin adapter: wasm-bindgen -> core (cdylib, wasm32 target)
  tests/            ← JS integration tests only (run via `wasm-pack build` + node)
```

## Non-obvious rules

- **Comments**: no `—` (use `:`), no `====` divider blocks, no `See:` or `[...](..)` doc links, lowercase prose unless naming a type/tag/variable
- **Amounts**: always `Decimal`, always absolute; sign in `debit_credit` enum field
- **Dates**: always `chrono::NaiveDate` (no timezone); century cutoff `YY < 80 → 20YY`
- **Swift decoder keywords**: 3-4 uppercase ASCII letters (not just 4; BIC=3)
- **Import order**: std → external crates → `crate::` internal
- **Error types**: `thiserror`, return `Result<T, ParseError>`, no bare `unwrap()` in production code, prefer `?`
- **Tag 86 multi-line**: strip `\n` with NO space injection
- **:61: continuation lines**: tokenizer inserts `\n` separator; parser extracts into `Transaction.supplementary`
- **JSON field naming**: `#[allow(non_snake_case)]` on serialization structs (camelCase for JSON output)

## Design invariants (do not change without discussion)

1. All logic in `crates/core`; other crates are thin adapters
2. Amounts absolute + `debit_credit`; never `f64`/`f32`
3. Tag 86 auto-detection per-transaction, not per-statement
4. Resolver is priority hint, not lock; `UnstructuredDecoder` always last in chain
5. Resolver values: `auto`, `swift`, `gvc`, `angular` (unified across all bindings)
6. camt.053: amounts always absolute, sign in `CdtDbtInd`; D→`Cdtr`, C→`Dbtr`
7. Mandatory tags validated at statement-finalize time: `:25:`, `:28C:`, `:60F:`, `:62F:`

## CI

| Workflow | Trigger | What |
|----------|---------|------|
| `test-core.yml` | push master, PR any branch | `cargo test -p x940rs -p x940` + proptests |
| `test-cli.yml` | push master, PR any branch | CLI integration tests (3 OS matrix) |
| `test-node.yml` | push master, PR any branch | Rust unit + JS integration (3 OS × 2 Node matrix) |
| `test-python.yml` | push master, PR any branch | pytest (3 OS × 3 Python matrix: 3.10-3.12) |
| `test-wasm.yml` | push master, PR any branch | JS integration via wasm-pack (3 OS × 2 Node matrix) |
| `check-format.yml` | PR any branch | clippy, fmt, doc, build, 4-way version sync |
| `tag.yml` | manual dispatch | verify 4 versions match input, create git tag |
| `release.yml` | tag push / manual dispatch | tests → cargo publish + wheel build + PyPI publish + npm publish (native + wasm) |

## Versioning

All packages share one version in `Cargo.toml` (all Rust crates inherit via
`version.workspace = true`). Use `./bump.sh 0.2.0` to sync `Cargo.toml`,
`crates/python/pyproject.toml`, `crates/node/package.json`, and
`crates/wasm/package.json` at once.
The `check-format.yml` CI enforces they stay in sync on every PR.

## Adding a new Tag 86 dialect

1. `crates/core/src/decoders/new_dialect.rs`: impl `Tag86Decoder`
2. Add `pub mod` to `decoders/mod.rs`
3. Insert into `DecoderChain::auto()` chain (before `UnstructuredDecoder`)
4. Add variant to `DecoderChain::with_resolver()` match arms
5. `tests/data/new_dialect/`: `.sta` payload + expected `.json` golden file
6. `crates/python/tests/test_dialects.py`: Python-side dialect test
7. `crates/node/tests/test_auto_detect.js`: Node.js dialect test
8. `crates/wasm/tests/test_auto_detect.js`: WASM dialect test
