# x940: Technical Architecture

## 1. Overview

x940 is a **Cargo workspace** containing four crates:

```mermaid
graph TD
    subgraph "Cargo Workspace"
        core["x940rs<br/>(Rust engine)"]
        py["x940py<br/>(PyO3 binding)"]
        cli["x940<br/>(CLI binary)"]
        node["x940node<br/>(napi-rs binding)"]
    end

    py -->|"depends on"| core
    cli -->|"depends on"| core
    node -->|"depends on"| core

    subgraph "External"
        python["Python<br/>(pip install x940)"]
        terminal["Terminal<br/>(x940 transform)"]
        npm["Node.js<br/>(npm install x940)"]
        rustlib["Rust<br/>(cargo add x940rs)"]
    end

    python -->|"imports"| py
    terminal -->|"runs"| cli
    npm -->|"requires"| node
    rustlib -->|"uses"| core
```

| Crate       | Role                                            |
|-------------|-------------------------------------------------|
| `x940rs`    | All business logic: parsing, models, decoders, serializers |
| `x940py`    | PyO3 binding: exposes `MT940` class to Python   |
| `x940`      | CLI binary via `clap`                           |
| `x940node`  | napi-rs binding: native Node.js addon           |

**Golden rule**: All business logic lives in `crates/core`. Every other crate
is a thin adapter. A bug fix in `core` benefits all consumers automatically.

## 2. Data Flow

```mermaid
flowchart LR
    input["Raw MT940 text<br/>(file, string, stream)"]
    parser["Parser<br/>Line Tokenizer -> FSM<br/>-> Tag86 Decoder Chain"]
    models["Structured Data Models<br/>Vec&lt;Statement&gt;"]
    json["JSON<br/>(serde_json)"]
    csv["CSV<br/>(manual builder)"]
    xml["camt.053 XML<br/>(quick-xml)"]

    input --> parser
    parser -->|"per-transaction<br/>auto-detection"| models
    models --> json
    models --> csv
    models --> xml

    json --> file_out[".json"]
    csv --> file_out2[".csv"]
    xml --> file_out3[".xml"]
```

## 3. Parser Architecture

### 3.1 Finite State Machine (FSM)

```mermaid
stateDiagram-v2
    [*] --> START
    START --> HEADER: :20:
    HEADER --> HEADER: :21:
    HEADER --> HEADER: :25:
    HEADER --> HEADER: :28C:
    HEADER --> BODY: :60F: or :60M:
    BODY --> BODY: :61:
    BODY --> BODY: :86:
    BODY --> FOOTER: :62F: or :62M:
    FOOTER --> FOOTER: :64:
    FOOTER --> FOOTER: :65:
    FOOTER --> FOOTER: :86: (standalone)
    FOOTER --> HEADER: :20: (next statement)
    FOOTER --> [*]
```

### 3.2 Tag 86 Decoder Chain (Per-Transaction Auto-Detection)

```mermaid
flowchart TD
    raw[":86: raw text block"]
    raw --> s1{"SwiftStructuredDecoder<br/>contains /KEYWORD/?"}
    s1 -->|"yes"| sd["StructuredDetails<br/>EREF, REMI, NAME, BIC, IBAN"]
    s1 -->|"no"| s2{"GermanGvcDecoder<br/>contains ?DD?"}
    s2 -->|"yes"| gvc["StructuredDetails<br/>gvc, 00, 20-29, 30-33"]
    s2 -->|"no"| s3{"AngularDecoder<br/>contains &lt;DD or ^DD?"}
    s3 -->|"yes"| ang["StructuredDetails<br/>tx_code, 00, 20, 27, 30"]
    s3 -->|"no"| s4["UnstructuredDecoder<br/>always matches"]
    s4 --> unstruct["StructuredDetails<br/>detail: raw_text"]
```

### 3.3 Resolver = Priority, Not Lock

```mermaid
flowchart LR
    subgraph "resolver=auto"
        a1[Swift] --> a2[GVC] --> a3[Angular] --> a4[Unstruct]
    end

    subgraph "resolver=gvc"
        g1[GVC] --> g2[Unstruct]
    end

    subgraph "resolver=swift"
        w1[Swift] --> w2[Unstruct]
    end
```

When an explicit resolver is passed, the chain is shortened to
`[ChosenDecoder, UnstructuredDecoder]`. Transactions that don't match the
chosen dialect fall through to the unstructured safety net: no parse
errors, no data loss.

### 3.4 Tag 86 Multi-Line Concatenation (No-Space Rule)

```mermaid
flowchart LR
    line1["TRANSACTION THAT\n"]
    line2["SHOULD CONTINUOUSLY PARSE"]
    concat["TRANSACTION THATSHOULD CONTINUOUSLY PARSE"]

    line1 -->|"strip newline<br/>no space injection"| concat
    line2 -->|""| concat
```

### 3.5 :61: Continuation Lines (SWIFT [34x])

Per the SWIFT specification, the `[34x]` supplementary details field can spill
onto continuation lines after `:61:` without a tag prefix. The tokenizer
inserts `\n` as a separator (unlike :86: which uses the no-space rule), and the
parser extracts the continuation text into `Transaction.supplementary`.

### 3.6 Two Representations in Transaction

```mermaid
flowchart TD
    tx[Transaction]
    tx --> details["details: String<br/>raw :86: text<br/>always preserved"]
    tx --> sd["structured_details: Option&lt;HashMap&gt;<br/>parsed key-value pairs<br/>when a decoder matches"]

    exporters[Exporters]
    exporters -->|"read first"| sd
    exporters -->|"fallback"| details
```

## 4. Key Design Decisions

### 4.1 Amount Storage: Absolute + Debit/Credit Flag

```mermaid
flowchart LR
    amt["raw amount: 1500,00"]
    dc["D/C mark: D"]
    store["store: amount=1500.00<br/>debit_credit=Debit"]
    display["display: signed_amount()<br/>returns -1500.00"]

    amt --> store
    dc --> store
    store --> display
```

| Export     | Amount Format    |
|------------|-----------------|
| JSON       | Signed (negative for D, RC) |
| CSV        | Signed (negative for D, RC) |
| camt.053   | Absolute (sign in CdtDbtInd) |

### 4.2 Reversal Handling

```mermaid
flowchart LR
    d[":61: D"] --> store_d["Debit<br/>money leaves account"]
    c[":61: C"] --> store_c["Credit<br/>money enters account"]
    rd[":61: RD"] --> store_rd["ReversalDebit<br/>treated as credit"]
    rc[":61: RC"] --> store_rc["ReversalCredit<br/>treated as debit"]

    store_rd --> camt["camt.053<br/>RvslInd=true"]
    store_rc --> camt
```

### 4.3 Trait-Based Decoders

| Format                | Region                 | Delimiter | Decoder                 |
|-----------------------|------------------------|-----------|------------------------|
| SWIFT structured      | International, SEPA    | `/`       | `SwiftStructuredDecoder` |
| German GVC            | Germany, Austria, CH   | `?`       | `GermanGvcDecoder`       |
| Angular (Polish)      | Poland, Czechia        | `<`       | `AngularDecoder`         |
| Angular (Nordic)      | Nordics                | `^`       | `AngularDecoder`         |
| Unstructured          | US, Asia, legacy       | (none)    | `UnstructuredDecoder`    |

### 4.4 Multi-Statement Handling

The Rust API returns `Vec<Statement>`: multiple statements per file. The
Python and Node.js `MT940` classes wrap the full vector. Default accessors
return the first statement's values for convenience.

## 5. Output Formats

### 5.1 JSON

- Top-level JSON array of statement objects
- Signed amounts (negative for debits)
- `structuredDetails` always present (at minimum `{"detail": raw_text}`)
- Statement-level fields included

### 5.2 CSV

- Flattened: one row per transaction
- UTF-8 BOM for Excel compatibility
- Signed amounts (negative for debits)
- Columns: Statement, Account, Currency, Date, EntryDate, Type, Reference,
  BankRef, Counterparty, CounterIBAN, Purpose, Amount, IsReversal

### 5.3 camt.053 XML

```mermaid
flowchart TD
    tx[":61: Transaction"]
    tx -->|"D (Debit)"| dout["CdtDbtInd=DBIT<br/>counterparty in Cdtr (Creditor)"]
    tx -->|"C (Credit)"| cout["CdtDbtInd=CRDT<br/>counterparty in Dbtr (Debtor)"]
    tx -->|"RD/RC"| rev["RvslInd=true"]

    subgraph "amount handling"
        abs["all amounts absolute<br/>(sign in CdtDbtInd)"]
    end
```

- Target: `camt.053.001.06`
- All amounts are absolute (positive): sign in CdtDbtInd
- Debit to Cdtr routing, credit to Dbtr routing
- NtryDtls always included (minimum: RmtInf with Ustrd)

## 6. Technology Stack

```mermaid
graph LR
    subgraph "Core Engine"
        rust[Rust]
        regex[regex]
        decimal[rust_decimal]
        chrono[chrono]
    end

    subgraph "Serialization"
        serde[serde_json]
        quickxml[quick-xml]
    end

    subgraph "Bindings"
        pyo3[PyO3 + Maturin]
        clap[clap]
        napi[napi-rs]
        thiserror[thiserror]
    end

    rust --> regex
    rust --> decimal
    rust --> chrono
    rust --> serde
    rust --> quickxml
    rust --> pyo3
    rust --> clap
    rust --> napi
    rust --> thiserror
```

| Layer              | Technology          | Purpose                        |
|--------------------|---------------------|--------------------------------|
| Core parsing       | Rust + regex        | Tokenization, FSM, tag parsing |
| Decimal math       | `rust_decimal`      | Exact financial amounts        |
| Dates              | `chrono`            | Date parsing and formatting    |
| JSON output        | `serde_json`        | Serialize models to JSON       |
| XML output         | `quick-xml`         | camt.053 XML generation        |
| Python binding     | `pyo3` + `maturin`  | Native CPython extension       |
| Node.js binding    | `napi-rs`           | Native Node addon              |
| CLI                | `clap`              | Argument parsing               |
| Error handling     | `thiserror`         | Ergonomic error types           |

## 7. Testing Strategy

### 7.1 Three-Tier Rust Tests

```mermaid
flowchart TD
    tier1["Tier 1: Inline<br/>#[cfg(test)] mod tests<br/>private functions"]
    tier2["Tier 2: Integration<br/>tests/ directory<br/>public API only"]
    tier3["Tier 3: Property-based<br/>proptest<br/>random input fuzzing"]

    tier1 -->|"33 unit"| ci
    tier2 -->|"28 integration"| ci
    tier3 -->|"6 proptests"| ci

    ci["CI: cargo test"]
```

### 7.2 Test Coverage

| Layer            | Tests | Framework    |
|------------------|-------|--------------|
| Rust unit        | 33    | `#[cfg(test)]` |
| Rust integration | 28    | `tests/`       |
| Rust proptest    | 6     | proptest       |
| Python           | 39    | pytest         |
| Node.js (Rust)   | 8     | `#[cfg(test)]` |
| Node.js (JS)     | 35    | Node.js assert |
| CLI              | 9     | assert_cmd     |
| Doc tests        | 3     | rustdoc        |

### 7.3 Golden File Testing

Six `.sta` payloads in `tests/data/` across all four dialect types plus a
SWIFT continuation-line test. Tests parse each file and assert structured
output matches expectations.

## 8. Bindings Architecture

```mermaid
flowchart TD
    core["crates/core<br/>Rust<br/>parse_mt940()<br/>to_json() to_csv() to_camt053()"]
    core -->|"PyO3 FFI"| py["crates/python<br/>x.MT940(data)"]
    core -->|"native"| cli["crates/cli<br/>x940 transform"]
    core -->|"napi-rs"| node["crates/node<br/>new MT940(data)"]

    py --> python_user["import x940 as x"]
    cli --> terminal_user["x940 transform input.sta"]
    node --> ts_user["import { MT940 } from 'x940'"]
```

## 9. Known Limitations Addressed

x940 fixes the following defects found in the reference parser
(eu-invoice-tools):

| Limitation                    | x940 Fix                                       |
|-------------------------------|------------------------------------------------|
| Polish angular not parsed     | Native `AngularDecoder` implementation         |
| No structuredDetails for unstructured | Always include `{"detail": raw_text}`  |
| CSV multiline truncation      | Full concatenation before CSV serialization    |
| camt.053 missing NtryDtls     | Always include NtryDtls with RmtInf            |
| GBP amounts lose precision    | Always output 2 decimal places                 |
| :61: continuation lines       | SWIFT [34x] supplementary details parsed       |
