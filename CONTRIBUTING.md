# Contributing to x940

## Getting Started

```bash
git clone git@github.com:tahzeer/x940.git
cd x940

cargo build
cargo test -p x940rs -p x940            # skip python crate (needs libpython3.12)
cargo clippy --workspace -- -D warnings
cargo fmt --check
```

`cargo test --workspace` fails without Python dev headers. Use `-p x940rs -p x940`.

## Development Workflow

1. Fork or create a feature branch
2. Write code in the appropriate crate:
   - Parser / models / decoders / serializers → `crates/core/`
   - CLI interface → `crates/cli/`
   - Python binding → `crates/python/`
   - Node.js binding → `crates/node/`
   - WASM binding → `crates/wasm/`
3. Add tests:
   - Inline unit tests: `#[cfg(test)] mod tests` in the same file
   - Core integration tests: `crates/core/tests/`
   - Python tests: `crates/python/tests/`
   - Node.js tests: `crates/node/tests/` (JS files, run via `npm test`)
   - WASM tests: `crates/wasm/tests/` (JS files, run via `npm test` after wasm-pack build)
4. Run the full suite:
   ```bash
   cargo test -p x940rs -p x940 -p x940node
   cargo fmt --check
   cargo clippy --workspace -- -D warnings
   cd crates/node && npm test
   ```
5. Commit and create a pull request

## Code Style

- Follow `rustfmt.toml` and `.editorconfig` in the repo root
- Read [AGENTS.md](AGENTS.md) for full coding conventions
- New public items must have doc comments (`///`)
- Write tests for all new functionality
- Use `thiserror` for error types; avoid bare `unwrap()` in production code

## Commit Conventions

- Prefix commits with the crate: `core:`, `cli:`, `python:`, `node:`, `wasm:`, `docs:`
- Example: `core: implement :61: continuation line parsing`
- Example: `python: add Transaction.counterparty property`
- Example: `node: add JS integration tests`
- Example: `wasm: add WASM bindings + CI`
- Keep commits small and focused

## Pull Requests

- Reference the issue being fixed (if applicable)
- Include a summary of changes
- All tests must pass before requesting review
- New features should include:
  - Rust code + tests
  - Binding updates (Python and/or Node.js, if applicable)
  - Documentation updates (README, USAGE, TECHNICAL, MT940 as needed)
  - Version bump via `./bump.sh` if it's a release PR

## Python Binding Development

```bash
uv pip install maturin pytest
uv run maturin develop -m crates/python/Cargo.toml
uv run pytest crates/python/tests/ -v
```

## Node.js Binding Development

```bash
cd crates/node
npm install
napi build --platform --release
npm test
```

## WASM Binding Development

```bash
rustup target add wasm32-unknown-unknown
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
cd crates/wasm
npm test
```

## Adding a New Export Format

1. Add serializer in `crates/core/src/serializers/`
2. Expose via `pub fn` in `crates/core/src/lib.rs`
3. Python: add `fn to_<format>` in `crates/python/src/lib.rs`
4. Node: add `#[napi] fn to_<format>` in `crates/node/src/lib.rs`
5. WASM: add `fn to_<format>` in `crates/wasm/src/lib.rs`
6. CLI: add `--format <format>` variant in `crates/cli/src/main.rs`
7. Write golden file tests with expected output
8. Update README.md and TECHNICAL.md
9. Update USAGE.md with examples for all bindings

## Adding a New Binding

1. Create `crates/<lang>/Cargo.toml` with `x940rs = { path = "../core" }`
2. Implement the `MT940` class/struct wrapping `parse_mt940` + `to_json`/`to_csv`/`to_camt053`
3. Add to workspace `members` in root `Cargo.toml`
4. Add tests (inline Rust + language-level integration)
5. Add CI workflow in `.github/workflows/test-<lang>.yml`
6. Add job to `.github/workflows/release.yml` for npm publishing
7. Add to `bump.sh` version sync
8. Add build-from-source instructions in README.md
9. Add usage examples in USAGE.md

## Reporting Issues

Use [GitHub Issues](https://github.com/tahzeer/x940/issues). Include:
- MT940 file snippet (anonymized if necessary)
- Expected behavior vs actual behavior
- Version of x940 being used
