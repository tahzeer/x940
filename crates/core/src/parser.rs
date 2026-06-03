use chrono::{Datelike, NaiveDate};
use regex::Regex;
use rust_decimal::Decimal;
use std::sync::LazyLock;

use crate::decoders::DecoderChain;
use crate::error::{ParseError, Result};
use crate::models::{Balance, DebitOrCredit, StatementNumber, Transaction};
use crate::statement::Statement;

// amount / date helpers

fn parse_amount(raw: &str) -> std::result::Result<Decimal, ParseError> {
    let n = raw.replace(',', ".");
    let parts: Vec<&str> = n.split('.').collect();
    let int = parts[0].trim_start_matches('0');
    let int = if int.is_empty() { "0" } else { int };
    let dec = if parts.len() > 1 { parts[1] } else { "" };
    let padded = format!("{:0<2}", dec);
    format!("{}.{}", int, padded).parse::<Decimal>().map_err(|_| ParseError::InvalidAmount {
        value: raw.to_string(),
        tag: "(amount field)",
    })
}

fn parse_date(raw: &str) -> std::result::Result<NaiveDate, ParseError> {
    if raw.len() != 6 {
        return Err(ParseError::InvalidDate {
            value: raw.to_string(),
            tag: "(date field)",
        });
    }
    let yy: i32 = raw[0..2].parse().map_err(|_| ParseError::InvalidDate {
        value: raw.to_string(),
        tag: "(date field)",
    })?;
    let year = if yy < 80 { 2000 + yy } else { 1900 + yy };
    let m: u32 = raw[2..4].parse().map_err(|_| ParseError::InvalidDate {
        value: raw.to_string(),
        tag: "(date field)",
    })?;
    let d: u32 = raw[4..6].parse().map_err(|_| ParseError::InvalidDate {
        value: raw.to_string(),
        tag: "(date field)",
    })?;
    NaiveDate::from_ymd_opt(year, m, d).ok_or(ParseError::InvalidDate {
        value: raw.to_string(),
        tag: "(date field)",
    })
}

fn infer_entry_date(value_date: NaiveDate, mmdd: &str) -> Option<NaiveDate> {
    let m: u32 = mmdd[0..2].parse().ok()?;
    let d: u32 = mmdd[2..4].parse().ok()?;
    let mut year = value_date.year();
    if value_date.month() == 12 && m < 6 {
        year += 1;
    }
    NaiveDate::from_ymd_opt(year, m, d)
}

// balance parsing (:60F:, :60M:, :62F:, :62M:, :64:, :65:)

fn parse_balance(value: &str, is_intermediate: bool) -> std::result::Result<Balance, ParseError> {
    if value.len() < 11 {
        return Err(ParseError::InvalidFormat {
            tag: "balance",
            value: value.to_string(),
            reason: "too short".into(),
        });
    }

    let dc = &value[0..1];
    let debit_credit = match dc {
        "C" => DebitOrCredit::Credit,
        "D" => DebitOrCredit::Debit,
        _ => {
            return Err(ParseError::InvalidFormat {
                tag: "balance",
                value: value.to_string(),
                reason: "invalid D/C mark".into(),
            })
        }
    };

    let date = parse_date(&value[1..7])?;
    let currency = value[7..10].to_string();

    let amount_str = &value[10..];
    let amount = parse_amount(amount_str)?;

    Ok(Balance {
        is_intermediate,
        debit_credit,
        date,
        currency,
        amount,
    })
}

// statement number parsing (:28C:)

fn parse_statement_number(value: &str) -> StatementNumber {
    if let Some(idx) = value.find('/') {
        StatementNumber {
            statement: value[..idx].to_string(),
            sequence: Some(value[idx + 1..].to_string()),
        }
    } else {
        StatementNumber {
            statement: value.to_string(),
            sequence: None,
        }
    }
}

// transaction parsing (:61:)

static TAG61_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(\d{6})(\d{4})?(RD|RC|D|C)([A-Z])?(\d{1,15},\d{0,2})([NF][A-Za-z0-9]{3})([^/]*?)(?://([^\r\n]*))?$",
    )
    .unwrap()
});

fn parse_transaction(value: &str) -> std::result::Result<Transaction, ParseError> {
    // Split at first \n to separate main :61: value from supplementary details.
    // Supplementary details ([34x]) appear on continuation lines per SWIFT spec.
    let (main_value, supplementary) = if let Some(nl) = value.find('\n') {
        let supp = value[nl + 1..].trim().to_string();
        (&value[..nl], if supp.is_empty() { None } else { Some(supp) })
    } else {
        (value, None)
    };

    let caps = TAG61_RE.captures(main_value).ok_or_else(|| ParseError::InvalidFormat {
        tag: ":61:",
        value: main_value.to_string(),
        reason: "does not match expected format".into(),
    })?;

    let vdate = parse_date(caps.get(1).unwrap().as_str())?;

    let entry_date = caps.get(2).map(|m| m.as_str()).and_then(|mmdd| {
        if mmdd.len() == 4 {
            infer_entry_date(vdate, mmdd)
        } else {
            None
        }
    });

    let dc_mark = caps.get(3).unwrap().as_str();
    let (debit_credit, funds_code) = match dc_mark {
        "RD" => (DebitOrCredit::ReversalDebit, None),
        "RC" => (DebitOrCredit::ReversalCredit, None),
        "D" | "C" => {
            let d = if dc_mark == "D" { DebitOrCredit::Debit } else { DebitOrCredit::Credit };
            let fc = caps.get(4).map(|m| m.as_str().to_string());
            (d, fc)
        }
        _ => unreachable!(),
    };

    let amount = parse_amount(caps.get(5).unwrap().as_str())?;
    let tx_type = caps.get(6).unwrap().as_str().to_string();
    let cust_ref = caps.get(7).map_or("", |m| m.as_str()).trim().to_string();
    let bank_ref = caps.get(8).map(|m| m.as_str().to_string());

    Ok(Transaction {
        value_date: vdate,
        entry_date,
        debit_credit,
        funds_code,
        amount,
        transaction_type: tx_type,
        customer_reference: cust_ref,
        bank_reference: bank_ref,
        supplementary,
        details: String::new(),
        structured_details: None,
    })
}

// line tokenizer

struct TagLine {
    tag: String,
    value: String,
}

fn tokenize_lines(input: &str) -> Vec<TagLine> {
    let mut lines = Vec::new();
    let mut current_tag = String::new();
    let mut current_value = String::new();

    let raw_lines: Vec<&str> = input.lines().collect();

    for line in raw_lines {
        let trimmed = line.trim_end_matches('\r');

        // Check if this line starts a new tag
        if let Some(stripped) = trimmed.strip_prefix(':') {
            if let Some(colon_idx) = stripped.find(':') {
                let tag = format!(":{}:", &stripped[..colon_idx]);

                // Push previous tag if any
                if !current_tag.is_empty() {
                    lines.push(TagLine {
                        tag: std::mem::take(&mut current_tag),
                        value: std::mem::take(&mut current_value),
                    });
                }

                current_tag = tag;
                current_value = stripped[colon_idx + 1..].to_string();
                continue;
            }
        }

        // Continuation line: belongs to previous tag.
        // :86: strips newline with no space (no-space rule).
        // :61: inserts newline to separate supplementary details.
        if !current_tag.is_empty() {
            if current_tag == ":61:" {
                current_value.push('\n');
            }
            current_value.push_str(trimmed);
        }
    }

    // Push final tag
    if !current_tag.is_empty() {
        lines.push(TagLine {
            tag: current_tag,
            value: current_value,
        });
    }

    lines
}

fn validate_statement(stmt: &Statement) -> Result<()> {
    if stmt.account_identification.is_empty() {
        return Err(ParseError::MissingTag {
            tag: ":25:",
            context: stmt.transaction_reference.clone(),
        });
    }
    if stmt.statement_number.statement.is_empty() {
        return Err(ParseError::MissingTag {
            tag: ":28C:",
            context: stmt.transaction_reference.clone(),
        });
    }
    if stmt.opening_balance.amount.is_zero() {
        return Err(ParseError::MissingTag {
            tag: ":60F:",
            context: stmt.transaction_reference.clone(),
        });
    }
    if stmt.closing_balance.amount.is_zero() {
        return Err(ParseError::MissingTag {
            tag: ":62F:",
            context: stmt.transaction_reference.clone(),
        });
    }
    Ok(())
}

// FSM parser

pub fn parse_mt940(input: &str, chain: &DecoderChain) -> Result<Vec<Statement>> {
    let tag_lines = tokenize_lines(input);

    if tag_lines.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    #[derive(PartialEq)]
    enum State {
        Start,
        Header,
        Body,
        Footer,
    }

    let mut state = State::Start;
    let mut statements: Vec<Statement> = Vec::new();
    let mut current: Option<Statement> = None;
    let mut transactions: Vec<Transaction> = Vec::new();
    let mut current_tx: Option<Transaction> = None;

    let push_statement = |s: &mut Option<Statement>, stmts: &mut Vec<Statement>| -> Result<()> {
        if let Some(stmt) = s.take() {
            validate_statement(&stmt)?;
            stmts.push(stmt);
        }
        Ok(())
    };

    for tl in &tag_lines {
        let tag = tl.tag.as_str();
        let val = tl.value.as_str();

        match state {
            State::Start => {
                if tag == ":20:" {
                    current = Some(Statement {
                        transaction_reference: val.to_string(),
                        related_reference: None,
                        account_identification: String::new(),
                        statement_number: StatementNumber {
                            statement: String::new(),
                            sequence: None,
                        },
                        opening_balance: Balance {
                            is_intermediate: false,
                            debit_credit: DebitOrCredit::Credit,
                            date: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                            currency: String::new(),
                            amount: Decimal::new(0, 0),
                        },
                        closing_balance: Balance {
                            is_intermediate: false,
                            debit_credit: DebitOrCredit::Credit,
                            date: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                            currency: String::new(),
                            amount: Decimal::new(0, 0),
                        },
                        closing_available: None,
                        forward_available: None,
                        transactions: Vec::new(),
                        info_to_owner: None,
                    });
                    transactions = Vec::new();
                    state = State::Header;
                }
            }

            State::Header => {
                match tag {
                    ":21:" => {
                        if let Some(ref mut s) = current {
                            s.related_reference = Some(val.to_string());
                        }
                    }
                    ":25:" => {
                        if let Some(ref mut s) = current {
                            s.account_identification = val.to_string();
                        }
                    }
                    ":28C:" => {
                        if let Some(ref mut s) = current {
                            s.statement_number = parse_statement_number(val);
                        }
                    }
                    ":60F:" | ":60M:" => {
                        if let Some(ref mut s) = current {
                            s.opening_balance = parse_balance(val, tag == ":60M:")?;
                        }
                        // Finalize any pending transaction before moving to body
                        if let Some(tx) = current_tx.take() {
                            transactions.push(tx);
                        }
                        state = State::Body;
                    }
                    _ => {}
                }
            }

            State::Body => {
                match tag {
                    ":61:" => {
                        // Finalize previous transaction
                        if let Some(tx) = current_tx.take() {
                            transactions.push(tx);
                        }
                        let tx = parse_transaction(val)?;
                        // Store parsed structured details immediately for :86: to use
                        current_tx = Some(tx);
                    }
                    ":86:" => {
                        // Resolve structured details through the decoder chain
                        let sd = chain.decode(val);

                        if let Some(tx) = current_tx.as_mut() {
                            tx.details = val.to_string();
                            tx.structured_details = Some(sd);
                        }
                    }
                    ":62F:" | ":62M:" => {
                        // Finalize last transaction
                        if let Some(tx) = current_tx.take() {
                            transactions.push(tx);
                        }
                        if let Some(ref mut s) = current {
                            s.closing_balance = parse_balance(val, tag == ":62M:")?;
                            s.transactions = std::mem::take(&mut transactions);
                        }
                        state = State::Footer;
                    }
                    _ => {}
                }
            }

            State::Footer => {
                match tag {
                    ":64:" => {
                        if let Some(ref mut s) = current {
                            s.closing_available = Some(parse_balance(val, false)?);
                        }
                    }
                    ":65:" => {
                        if let Some(ref mut s) = current {
                            s.forward_available = Some(parse_balance(val, false)?);
                        }
                    }
                    ":86:" => {
                        // Standalone info to owner in footer
                        if let Some(ref mut s) = current {
                            s.info_to_owner = Some(val.to_string());
                        }
                    }
                    ":20:" => {
                        push_statement(&mut current, &mut statements)?;
                        // Start new statement
                        current = Some(Statement {
                            transaction_reference: val.to_string(),
                            related_reference: None,
                            account_identification: String::new(),
                            statement_number: StatementNumber {
                                statement: String::new(),
                                sequence: None,
                            },
                            opening_balance: Balance {
                                is_intermediate: false,
                                debit_credit: DebitOrCredit::Credit,
                                date: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                                currency: String::new(),
                                amount: Decimal::new(0, 0),
                            },
                            closing_balance: Balance {
                                is_intermediate: false,
                                debit_credit: DebitOrCredit::Credit,
                                date: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                                currency: String::new(),
                                amount: Decimal::new(0, 0),
                            },
                            closing_available: None,
                            forward_available: None,
                            transactions: Vec::new(),
                            info_to_owner: None,
                        });
                        transactions = Vec::new();
                        state = State::Header;
                    }
                    _ => {}
                }
            }
        }
    }

    // Push final statement
    push_statement(&mut current, &mut statements)?;

    if statements.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    Ok(statements)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DecoderChain;

    #[test]
    fn tokenizes_simple_mt940() {
        let input = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n";
        let lines = tokenize_lines(input);
        assert!(lines.len() >= 5);
        assert_eq!(lines[0].tag, ":20:");
        assert_eq!(lines[1].tag, ":25:");
    }

    #[test]
    fn parses_minimal_statement() {
        let raw = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n";
        let chain = DecoderChain::auto();
        let stmts = parse_mt940(raw, &chain).unwrap();
        assert_eq!(stmts.len(), 1);
        let s = &stmts[0];
        assert_eq!(s.transaction_reference, "TEST");
        assert_eq!(s.account_identification, "ACCT");
    }

    #[test]
    fn parses_swift_payload() {
        let raw = std::fs::read_to_string("tests/data/swift/swift_payload_1.sta").unwrap();
        let chain = DecoderChain::auto();
        let stmts = parse_mt940(&raw, &chain).unwrap();
        assert_eq!(stmts.len(), 1);
        let s = &stmts[0];
        assert_eq!(s.transaction_reference, "SWIFTSTRUCT2026");
        assert_eq!(s.transactions.len(), 3);
    }

    #[test]
    fn parses_continuation_lines() {
        let raw =
            std::fs::read_to_string("tests/data/swift/continuation_payload.sta").unwrap();
        let chain = DecoderChain::auto();
        let stmts = parse_mt940(&raw, &chain).unwrap();
        let s = &stmts[0];
        assert_eq!(s.transactions.len(), 2);
        // First transaction has supplementary details on continuation line
        let tx = &s.transactions[0];
        assert_eq!(tx.transaction_type, "NTRF");
        assert!(tx.supplementary.is_some());
        assert!(tx.supplementary.as_ref().unwrap().contains("SUPPLEMENTARY DETAILS"));
        assert!(tx.supplementary.as_ref().unwrap().contains("FROM THE BANK"));
    }
}
