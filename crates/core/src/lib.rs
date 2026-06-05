//!
//! **x940rs**: High-performance MT940 bank statement parser
//!
//! This crate contains the complete business logic for parsing SWIFT MT940
//! Customer Statement Messages and converting them to modern structured formats
//! (JSON, CSV, camt.053 XML).
//!
//! # Architecture
//!
//! The parser operates as a line-by-line finite state machine (FSM) that
//! tokenizes MT940 tags and dispatches Tag 86 content to dialect-specific
//! decoders using a trait-based plugin system.
//!
//! Dialect auto-detection runs **per-transaction**: a single MT940 file
//! can mix SWIFT-structured, German GVC, Angular, and unstructured
//! transactions, and each is decoded independently.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use x940rs::{parse_mt940, DecoderChain, to_json};
//!
//! let raw = std::fs::read_to_string("statement.sta").unwrap();
//! let chain = DecoderChain::auto();
//! let statements = parse_mt940(&raw, &chain).unwrap();
//!
//! let json = to_json(&statements).unwrap();
//! std::fs::write("output.json", json).unwrap();
//! ```

pub mod decoders;
pub mod error;
pub mod models;
pub mod parser;
pub mod serializers;
pub mod statement;

pub use self::decoders::DecoderChain;
pub use self::error::{ParseError, Result};
pub use self::models::{Balance, DebitOrCredit, StatementNumber, Transaction};
pub use self::parser::parse_mt940;
pub use self::serializers::{amount_to_f64, to_camt053, to_csv, to_json};
pub use self::statement::Statement;

#[cfg(test)]
mod proptests {
    use crate::{parse_mt940, to_json, DecoderChain};
    use proptest::prelude::*;

    fn wrap_tag61(tag61: &str) -> String {
        format!(
            ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n{}\r\n:86:x\r\n:62F:C240101EUR100,00\r\n",
            tag61
        )
    }

    fn value_date_str() -> impl Strategy<Value = String> {
        (0u32..80u32, 1u32..13u32, 1u32..29u32)
            .prop_map(|(y, m, d)| format!("{:02}{:02}{:02}", y % 100, m, d))
    }

    fn dc_mark() -> impl Strategy<Value = String> {
        prop_oneof![
            8 => Just("D".into()),
            8 => Just("C".into()),
            1 => Just("RD".into()),
            1 => Just("RC".into()),
        ]
    }

    fn amount_str() -> impl Strategy<Value = String> {
        (1u32..100000u32, 0u32..100u32).prop_map(|(int, frac)| {
            if frac == 0 {
                format!("{},00", int)
            } else if frac < 10 {
                format!("{},0{}", int, frac)
            } else {
                format!("{},{}", int, frac)
            }
        })
    }

    fn tx_type_str() -> impl Strategy<Value = String> {
        prop_oneof![
            9 => Just("NTRF".into()),
            1 => Just("NMSC".into()),
            1 => Just("NCHG".into()),
            1 => Just("FTRF".into()),
        ]
    }

    proptest! {
        #[test]
        fn tag61_never_panics(
            vdate in value_date_str(),
            dc in dc_mark(),
            amt in amount_str(),
            txtype in tx_type_str(),
            custref in proptest::string::string_regex("[A-Za-z0-9/\\-?:().',+ ]{1,16}").unwrap(),
        ) {
            let tag61 = format!(":61:{}{}{}{}{}", vdate, dc, amt, txtype, custref);
            let input = wrap_tag61(&tag61);
            let chain = DecoderChain::auto();
            let _ = parse_mt940(&input, &chain);
        }

        #[test]
        fn parse_then_serialize_produces_valid_json(
            amt_int in 1u32..100000u32,
        ) {
            let amount = format!("{},00", amt_int);
            let input = format!(
                ":20:T\r\n:25:A\r\n:28C:1/1\r\n:60F:C240101EUR{}\r\n:62F:C240101EUR{}\r\n",
                amount, amount
            );
            let chain = DecoderChain::auto();
            let stmts = parse_mt940(&input, &chain).unwrap();
            let json = to_json(&stmts).unwrap();
            let val: serde_json::Value = serde_json::from_str(&json).unwrap();
            prop_assert!(val.is_array());
        }

        #[test]
        fn signed_amount_matches_debit_credit(
            vdate in value_date_str(),
            dc in dc_mark(),
            amt_int in 1u32..10000u32,
        ) {
            let amount = format!("{},00", amt_int);
            let tag61 = format!(":61:{}{}{}NTRFREF", vdate, dc, amount);
            let input = wrap_tag61(&tag61);
            let chain = DecoderChain::auto();
            let stmts = parse_mt940(&input, &chain).unwrap();
            let tx = &stmts[0].transactions[0];
            if tx.debit_credit.is_debit() {
                prop_assert!(tx.signed_amount() < rust_decimal::Decimal::ZERO);
            } else {
                prop_assert!(tx.signed_amount() > rust_decimal::Decimal::ZERO);
            }
            prop_assert!(tx.amount > rust_decimal::Decimal::ZERO);
        }

        #[test]
        fn parse_with_bank_reference(
            vdate in value_date_str(),
            dc in dc_mark(),
            amt_int in 1u32..10000u32,
            bankref in proptest::string::string_regex("[A-Za-z0-9]{1,16}").unwrap(),
        ) {
            let amount = format!("{},00", amt_int);
            let tag61 = format!(":61:{}{}{}NTRF//{}", vdate, dc, amount, bankref);
            let input = wrap_tag61(&tag61);
            let chain = DecoderChain::auto();
            let stmts = parse_mt940(&input, &chain).unwrap();
            let tx = &stmts[0].transactions[0];
            prop_assert_eq!(tx.bank_reference.as_deref(), Some(bankref.as_str()));
        }

        #[test]
        fn tag86_decoder_chain_always_returns_non_empty(
            raw in proptest::string::string_regex("[ -~]{1,200}").unwrap(),
        ) {
            let chain = DecoderChain::auto();
            let result = chain.decode(&raw);
            prop_assert!(!result.is_empty());
        }

        #[test]
        fn parse_never_drops_transactions(
            tx_count in 1u32..10u32,
        ) {
            let mut input = String::from(":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n");
            for i in 0..tx_count {
                input.push_str(&format!(":61:2401012401D{:03},00NTRF//REF{:03}\r\n", i, i));
                input.push_str(&format!(":86:test transaction {}\r\n", i));
            }
            input.push_str(":62F:C240101EUR1000,00\r\n");
            let chain = DecoderChain::auto();
            let stmts = parse_mt940(&input, &chain).unwrap();
            let txns = &stmts[0].transactions;
            prop_assert_eq!(txns.len() as u32, tx_count);
            for tx in txns {
                prop_assert!(!tx.details.is_empty());
                prop_assert!(tx.structured_details.is_some());
            }
        }
    }
}
