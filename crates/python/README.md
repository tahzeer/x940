# x940

High-performance MT940 bank statement parser. Converts legacy SWIFT MT940 files
to JSON, CSV, and ISO 20022 camt.053 XML — built in Rust with Python bindings
via PyO3.

## Installation

```bash
pip install x940
```

Requires Python 3.8+.

## Quick Start

```python
import x940 as x

stmt = x.MT940(open("statement.sta").read())

# statement info
stmt.account           # "EUR8934567890123456"
stmt.currency          # "EUR"
stmt.opening_balance   # 50000.0
stmt.closing_balance   # 51500.75
stmt.resolver_used     # "auto"

# transactions
len(stmt.transactions)  # 3
tx = stmt.transactions[0]
tx.value_date          # "2026-06-01"
tx.amount              # -1500.0 (signed: negative for debits)
tx.debit_credit        # "D"
tx.is_debit            # True
tx.customer_reference  # "INV-2026-991"
tx.counterparty        # "ALPHA DIGITAL CORP"
tx.purpose             # "MONTHLY RETAINER FEES"

# export
stmt.to_json()         # JSON string
stmt.to_csv()          # CSV string with UTF-8 BOM
stmt.to_camt053()      # camt.053.001.06 XML
```

## Dialect Detection

Tag 86 (transaction details) comes in several regional formats. x940
auto-detects the dialect per-transaction using a chain of decoders:

| Resolver  | Format                        | Use case                    |
|-----------|-------------------------------|-----------------------------|
| `auto`    | Auto-detect (default)         | Mixed-source files          |
| `swift`   | `/KEYWORD/VALUE` structured   | SWIFT-structured statements |
| `gvc`     | German `?DDKeyValue`          | German bank statements      |
| `angular` | Polish/Nordic `<DD` / `^DD`   | Polish, Nordic banks        |

Specify a resolver to prefer a specific dialect (unmatched transactions fall
back to unstructured):

```python
stmt = x.MT940(data, resolver="gvc")
stmt.resolver_used  # "gvc"
```

## Transaction Properties

| Property            | Type               | Description                            |
|---------------------|--------------------|----------------------------------------|
| `value_date`        | `str`              | Value date (YYYY-MM-DD)                |
| `entry_date`        | `str` or `None`    | Entry date (YYYY-MM-DD)                |
| `debit_credit`      | `str`              | `"D"` (debit), `"C"` (credit), etc.    |
| `amount`            | `float`            | Signed amount (negative = debit)       |
| `is_credit`         | `bool`             | True if this is a credit entry         |
| `is_debit`          | `bool`             | True if this is a debit entry          |
| `is_reversal`       | `bool`             | True if this is a reversal entry       |
| `transaction_type`  | `str`              | SWIFT transaction type code            |
| `customer_reference`| `str`              | Customer reference from :86:           |
| `bank_reference`    | `str` or `None`    | Bank reference                         |
| `details`           | `str`              | Raw Tag 86 detail text                 |
| `structured_details`| `dict` or `None`   | Parsed key-value map                   |
| `counterparty`      | `str`              | Resolved counterparty name             |
| `counter_iban`      | `str`              | Resolved counterparty IBAN             |
| `purpose`           | `str`              | Resolved remittance/purpose text       |
| `signed_amount()`   | `float`            | Same as `amount` property              |

## Export Formats

| Method          | Output                 | Description                        |
|-----------------|------------------------|------------------------------------|
| `to_json()`     | `str`                  | JSON array of statements           |
| `to_csv()`      | `str`                  | CSV with UTF-8 BOM, signed amounts |
| `to_camt053()`  | `str`                  | ISO 20022 camt.053.001.06 XML      |

## License

GPL-3.0-or-later. See [LICENSE](https://github.com/tahzeer/x940/blob/master/LICENSE).

## Links

- [Source & CLI](https://github.com/tahzeer/x940)
- [MT940 Format Reference](https://github.com/tahzeer/x940/blob/master/MT940.md)
- [Usage Guide](https://github.com/tahzeer/x940/blob/master/USAGE.md)
