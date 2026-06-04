# x940

High-performance MT940 bank statement parser for Node.js. Converts legacy SWIFT
MT940 files to JSON, CSV, and ISO 20022 camt.053 XML.

Built on the [x940](https://github.com/tahzeer/x940) Rust engine via napi-rs.

## Install

```bash
npm install x940
```

## Quick Start

```js
const { MT940 } = require("x940");
const fs = require("node:fs");

const data = fs.readFileSync("statement.sta", "utf8");
const stmt = new MT940(data, "auto");

console.log(stmt.account);
console.log(stmt.currency);
console.log(stmt.openingBalance.toFixed(2));

// Export
fs.writeFileSync("output.json", stmt.toJson());
fs.writeFileSync("output.csv",  stmt.toCsv());
fs.writeFileSync("output.xml",  stmt.toCamt053());

// Transaction data via JSON
const json = JSON.parse(stmt.toJson());
for (const tx of json[0].transactions) {
    console.log(`${tx.amount} ${tx.transactionType} ${tx.structuredDetails?.NAME}`);
}
```

## API

### `new MT940(text, resolver?)`

| Param | Type | Description |
|-------|------|-------------|
| `text` | string | Raw MT940 file content |
| `resolver` | string | Dialect: `"auto"` (default), `"swift"`, `"gvc"`, `"angular"` |

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `account` | string | Account identification (from :25:) |
| `currency` | string | ISO 4217 currency code |
| `openingBalance` | number | Opening balance amount |
| `closingBalance` | number | Closing balance amount |
| `resolverUsed` | string | Resolver used for Tag 86 decoding |

### Methods

| Method | Returns | Description |
|--------|---------|-------------|
| `toJson()` | string | Pretty-printed JSON array |
| `toCsv()` | string | CSV with UTF-8 BOM |
| `toCamt053()` | string | ISO 20022 camt.053.001.06 XML |

## Resolver Options

| Value | Description |
|-------|-------------|
| `auto` | Auto-detect per-transaction (default) |
| `swift` | SWIFT `/KEYWORD/VALUE` structured format |
| `gvc` | German `?DD` GVC format |
| `angular` | Polish/Nordic `<DD` / `^DD` format |

## Supported Platforms

Pre-built binaries for:

- Linux x64 (glibc)
- macOS x64 / arm64
- Windows x64

## License

GPL-3.0-or-later

[Repository](https://github.com/tahzeer/x940) ·
[Documentation](https://github.com/tahzeer/x940#readme) ·
[Issues](https://github.com/tahzeer/x940/issues)
