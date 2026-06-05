# x940

> **x940 is under active development** and may not yet be
> production-ready. APIs, output formats, and behavior may change between
> releases as the parser evolves to handle more MT940 dialects, edge cases,
> and SWIFT variations. Feedback, bug reports, and contributions are welcome.

High-performance MT940 bank statement parser. Converts legacy SWIFT MT940 files
to JSON, CSV, and ISO 20022 camt.053 XML.

Built in Rust with native Python, Node.js, WASM, and CLI bindings.

## Installation

### Python

```bash
pip install x940
```

### CLI

```bash
cargo install x940
```

### Node.js

```bash
npm install x940
```

### WASM

```bash
npm install x940-wasm
```

### Rust

```toml
[dependencies]
x940rs = "0.1.1"
```

## Build from Source

```bash
git clone git@github.com:tahzeer/x940.git
cd x940

# CLI: build and install the binary directly
cargo install --path crates/cli

# Rust library: add to your project via path
# Cargo.toml:  x940rs = { path = "/path/to/x940/crates/core" }

# Python binding: install in dev mode
uv pip install maturin
uv run maturin develop -m crates/python/Cargo.toml --release
uv run pytest crates/python/tests/ -v

# Node.js binding: build the native addon
cd crates/node && npm install && napi build --platform --release

# WASM binding: build with wasm-pack
rustup target add wasm32-unknown-unknown
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
cd crates/wasm && npm test

# Run tests
cargo test -p x940rs -p x940
uv run pytest crates/python/tests/ -v
cd crates/node && npm test

# Lint
cargo clippy --workspace -- -D warnings
cargo fmt --check

# Bump version across all packages
./bump.sh 0.2.0

# Benchmarks
cargo bench -p x940rs
```

## Quick Start

```python
import x940 as x

stmt = x.MT940(open("statement.sta").read())
stmt.to_json()     # -> JSON string
stmt.to_csv()      # -> CSV string with UTF-8 BOM
stmt.to_camt053()  # -> camt.053.001.06 XML
```

```bash
x940 transform statement.sta --format json --output result.json
x940 transform statement.sta --format csv  --output result.csv
cat statement.sta | x940 transform --format csv > txns.csv
```

```js
const { MT940 } = require("x940");
const stmt = new MT940(data, "auto");
stmt.toJSON();
stmt.toCSV();
stmt.toCamt053();
```

```js
// WASM (browser, Deno, Bun, Node.js)
import { MT940 } from "x940-wasm";
const stmt = new MT940(data, "auto");
stmt.toJson();
stmt.toCsv();
stmt.toCamt053();
```

```rust
use x940rs::{parse_mt940, DecoderChain, to_json};

let chain = DecoderChain::auto();
let statements = parse_mt940(&raw, &chain)?;
let json = to_json(&statements)?;
```

## Features

- **4 Tag 86 dialect decoders** with per-transaction auto-detection:
  SWIFT structured (`/KEYWORD/VALUE`), German GVC (`?DD`),
  Polish/Nordic angular (`<DD` / `^DD`), unstructured fallback
- **3 export formats**: JSON, CSV (UTF-8 BOM), camt.053.001.06 XML
- **SWIFT-compliant**: Handles `:61:` continuation lines, reversals (RD/RC), entry date inference
- **Exact decimal math** using `rust_decimal` (no IEEE 754 float drift)
- **GIL released during parse** for concurrent Python threads
- **Multi-statement files**: Handles multiple `:20:` blocks in one file

## Resolver Options

| Resolver   | Description                              |
|------------|------------------------------------------|
| `auto`     | Auto-detect per-transaction (default)    |
| `swift`    | SWIFT `/KEYWORD/VALUE` structured format |
| `gvc`      | German `?DD` GVC format                  |
| `angular`  | Polish/Nordic `<DD` / `^DD` format       |

Resolvers are priority hints, not hard locks. Unmatched transactions fall back
to the unstructured safety net: no data loss.

## Bindings

| Binding       | Technology        |
|---------------|-------------------|
| Rust          | Native crate      |
| Python        | PyO3 + Maturin    |
| CLI           | Native binary     |
| Node.js/TS    | napi-rs           |
| WASM          | wasm-bindgen      |

## License

GPL-3.0-or-later. See [LICENSE](LICENSE).

## References

- [MT940 Format Reference](MT940.md): SWIFT tag specifications, dialect formats, ZKA subfield codes
- [Usage Guide](USAGE.md): In-depth examples for Python, Node.js, CLI, and Rust
- [Technical Architecture](TECHNICAL.md): Design decisions, data flow, parser internals
