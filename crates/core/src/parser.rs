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

        if let Some(stripped) = trimmed.strip_prefix(':') {
            if let Some(colon_idx) = stripped.find(':') {
                let tag = format!(":{}:", &stripped[..colon_idx]);

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

        if !current_tag.is_empty() {
            if current_tag == ":61:" {
                current_value.push('\n');
            }
            current_value.push_str(trimmed);
        }
    }

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
    if !stmt.has_opening_balance {
        return Err(ParseError::MissingTag {
            tag: ":60F:",
            context: stmt.transaction_reference.clone(),
        });
    }
    if !stmt.has_closing_balance {
        return Err(ParseError::MissingTag {
            tag: ":62F:",
            context: stmt.transaction_reference.clone(),
        });
    }
    Ok(())
}

fn finalize_statement(
    current: &mut Option<Statement>,
    statements: &mut Vec<Statement>,
) -> Result<()> {
    if let Some(stmt) = current.take() {
        validate_statement(&stmt)?;
        statements.push(stmt);
    }
    Ok(())
}

// ParserState — bundles all FSM mutable state into one struct so handlers
// have a single &mut parameter, making them independently testable.

#[derive(Debug, PartialEq)]
pub(crate) enum State {
    Start,
    Header,
    Body,
    Footer,
}

pub(crate) struct ParserState {
    pub current: Option<Statement>,
    pub transactions: Vec<Transaction>,
    pub current_tx: Option<Transaction>,
    pub state: State,
}

impl Default for ParserState {
    fn default() -> Self {
        ParserState {
            current: None,
            transactions: Vec::new(),
            current_tx: None,
            state: State::Start,
        }
    }
}

// FSM state handlers

fn handle_header_tag(tag: &str, val: &str, ps: &mut ParserState) -> Result<()> {
    match tag {
        ":21:" => {
            if let Some(ref mut s) = ps.current {
                s.related_reference = Some(val.to_string());
            }
        }
        ":25:" => {
            if let Some(ref mut s) = ps.current {
                s.account_identification = val.to_string();
            }
        }
        ":28C:" => {
            if let Some(ref mut s) = ps.current {
                s.statement_number = parse_statement_number(val);
            }
        }
        ":60F:" | ":60M:" => {
            if let Some(ref mut s) = ps.current {
                s.set_opening_balance(parse_balance(val, tag == ":60M:")?);
            }
            if let Some(tx) = ps.current_tx.take() {
                ps.transactions.push(tx);
            }
            ps.state = State::Body;
        }
        _ => {}
    }
    Ok(())
}

fn handle_body_tag(tag: &str, val: &str, chain: &DecoderChain, ps: &mut ParserState) -> Result<()> {
    match tag {
        ":61:" => {
            if let Some(tx) = ps.current_tx.take() {
                ps.transactions.push(tx);
            }
            ps.current_tx = Some(parse_transaction(val)?);
        }
        ":86:" => {
            let sd = chain.decode(val);
            if let Some(tx) = ps.current_tx.as_mut() {
                tx.details = val.to_string();
                tx.structured_details = Some(sd);
            }
        }
        ":62F:" | ":62M:" => {
            if let Some(tx) = ps.current_tx.take() {
                ps.transactions.push(tx);
            }
            if let Some(ref mut s) = ps.current {
                s.set_closing_balance(parse_balance(val, tag == ":62M:")?);
                s.transactions = std::mem::take(&mut ps.transactions);
            }
            ps.state = State::Footer;
        }
        _ => {}
    }
    Ok(())
}

fn handle_footer_tag(tag: &str, val: &str, ps: &mut ParserState) -> Result<()> {
    match tag {
        ":64:" => {
            if let Some(ref mut s) = ps.current {
                s.closing_available = Some(parse_balance(val, false)?);
            }
        }
        ":65:" => {
            if let Some(ref mut s) = ps.current {
                s.forward_available = Some(parse_balance(val, false)?);
            }
        }
        ":86:" => {
            if let Some(ref mut s) = ps.current {
                s.info_to_owner = Some(val.to_string());
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn parse_mt940(input: &str, chain: &DecoderChain) -> Result<Vec<Statement>> {
    let tag_lines = tokenize_lines(input);

    if tag_lines.is_empty() {
        return Err(ParseError::EmptyInput);
    }

    let mut ps = ParserState::default();
    let mut statements: Vec<Statement> = Vec::new();

    for tl in &tag_lines {
        let tag = tl.tag.as_str();
        let val = tl.value.as_str();

        match ps.state {
            State::Start => {
                if tag == ":20:" {
                    ps.current = Some(Statement::new(val.to_string()));
                    ps.transactions = Vec::new();
                    ps.state = State::Header;
                }
            }

            State::Header => {
                handle_header_tag(tag, val, &mut ps)?;
            }

            State::Body => {
                handle_body_tag(tag, val, chain, &mut ps)?;
            }

            State::Footer => {
                if tag == ":20:" {
                    finalize_statement(&mut ps.current, &mut statements)?;
                    ps.current = Some(Statement::new(val.to_string()));
                    ps.transactions = Vec::new();
                    ps.state = State::Header;
                } else {
                    handle_footer_tag(tag, val, &mut ps)?;
                }
            }
        }
    }

    finalize_statement(&mut ps.current, &mut statements)?;

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
        let raw = std::fs::read_to_string("tests/data/swift/continuation_payload.sta").unwrap();
        let chain = DecoderChain::auto();
        let stmts = parse_mt940(&raw, &chain).unwrap();
        let s = &stmts[0];
        assert_eq!(s.transactions.len(), 2);
        let tx = &s.transactions[0];
        assert_eq!(tx.transaction_type, "NTRF");
        assert!(tx.supplementary.is_some());
        assert!(tx.supplementary.as_ref().unwrap().contains("SUPPLEMENTARY DETAILS"));
        assert!(tx.supplementary.as_ref().unwrap().contains("FROM THE BANK"));
    }

    // ParserState / handler unit tests

    #[test]
    fn handle_header_tag_parses_opening_balance() {
        let mut ps = ParserState {
            current: Some(Statement::new("T".into())),
            ..ParserState::default()
        };
        handle_header_tag(":60F:", "C240101EUR1500,00", &mut ps).unwrap();
        let s = ps.current.as_ref().unwrap();
        assert!(s.has_opening_balance);
        assert_eq!(s.opening_balance.amount.to_string(), "1500.00");
        assert_eq!(ps.state, State::Body);
    }

    #[test]
    fn handle_header_tag_parses_account_and_statement_number() {
        let mut ps = ParserState {
            current: Some(Statement::new("T".into())),
            ..ParserState::default()
        };
        handle_header_tag(":25:", "DE1234567890", &mut ps).unwrap();
        handle_header_tag(":28C:", "5/1", &mut ps).unwrap();
        let s = ps.current.as_ref().unwrap();
        assert_eq!(s.account_identification, "DE1234567890");
        assert_eq!(s.statement_number.statement, "5");
        assert_eq!(s.statement_number.sequence.as_deref(), Some("1"));
    }

    #[test]
    fn handle_body_tag_parses_transaction() {
        let chain = DecoderChain::auto();
        let mut ps = ParserState {
            current: Some(Statement::new("T".into())),
            ..ParserState::default()
        };
        handle_body_tag(":61:", "2401012401D100,00NTRF//REF001", &chain, &mut ps).unwrap();
        let tx = ps.current_tx.as_ref().unwrap();
        assert_eq!(tx.amount.to_string(), "100.00");
        assert_eq!(tx.transaction_type, "NTRF");
        assert_eq!(tx.bank_reference.as_deref(), Some("REF001"));
    }

    #[test]
    fn handle_body_tag_parses_closing_balance() {
        let chain = DecoderChain::auto();
        let mut ps = ParserState {
            current: Some(Statement::new("T".into())),
            ..ParserState::default()
        };
        handle_body_tag(":62F:", "C240101EUR900,00", &chain, &mut ps).unwrap();
        let s = ps.current.as_ref().unwrap();
        assert!(s.has_closing_balance);
        assert_eq!(s.closing_balance.amount.to_string(), "900.00");
        assert_eq!(ps.state, State::Footer);
    }

    #[test]
    fn handle_footer_tag_parses_available_balances() {
        let mut ps = ParserState {
            current: Some(Statement::new("T".into())),
            ..ParserState::default()
        };
        handle_footer_tag(":64:", "C240101EUR800,00", &mut ps).unwrap();
        handle_footer_tag(":65:", "C240131EUR700,00", &mut ps).unwrap();
        let s = ps.current.as_ref().unwrap();
        assert_eq!(s.closing_available.as_ref().unwrap().amount.to_string(), "800.00");
        assert_eq!(s.forward_available.as_ref().unwrap().amount.to_string(), "700.00");
    }

    #[test]
    fn validate_statement_rejects_missing_opening_balance() {
        let mut s = Statement::new("T".into());
        s.account_identification = "A".into();
        s.statement_number = StatementNumber {
            statement: "1".into(),
            sequence: None,
        };
        // has_opening_balance is false by default
        assert!(validate_statement(&s).is_err());
    }

    #[test]
    fn validate_statement_accepts_zero_balance() {
        let mut s = Statement::new("T".into());
        s.account_identification = "A".into();
        s.statement_number = StatementNumber {
            statement: "1".into(),
            sequence: None,
        };
        s.set_opening_balance(Balance::default()); // zero amount, but tag was parsed
        s.set_closing_balance(Balance::default());
        // Should pass even though amounts are zero
        assert!(validate_statement(&s).is_ok());
    }
}
