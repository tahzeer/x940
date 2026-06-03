//!
//! Integration tests: golden file parsing.
//!
//! These tests parse known MT940 files and verify structured output.
//! They verify the FSM parser, multi-line concatenation, and per-transaction
//! dialect auto-detection on full files.
//!
//! Tests are `#[ignore]`d until the FSM parser implementation is complete.
//! Run with: `cargo test -- --ignored`.

use std::fs;
use x940rs::{parse_mt940, to_json, DecoderChain};

/// Helper: parse a .sta file from tests/data/ and return JSON output.
fn parse_and_serialize(path: &str) -> String {
    let raw =
        fs::read_to_string(format!("tests/data/{}", path)).expect("test data file should exist");
    let chain = DecoderChain::auto();
    let statements = parse_mt940(&raw, &chain).expect("should parse valid MT940 file");
    to_json(&statements).expect("should serialize to JSON")
}

// payload 1: swift structured format

#[test]
fn golden_swift_structured_parses_three_transactions() {
    let json = parse_and_serialize("swift/swift_payload_1.sta");

    let parsed: serde_json::Value =
        serde_json::from_str(&json).expect("output should be valid JSON");

    assert!(parsed.is_array());
    let stmt = &parsed[0];

    // Statement-level fields
    assert_eq!(stmt["transactionReference"], "SWIFTSTRUCT2026");
    assert_eq!(stmt["accountIdentification"], "EUR8934567890123456");
    assert_eq!(stmt["currency"], "EUR");

    // Three transactions
    let txns = stmt["transactions"].as_array().expect("should have transactions");
    assert_eq!(txns.len(), 3);

    // First transaction: debit
    let tx1 = &txns[0];
    assert_eq!(tx1["transactionType"], "NTRF");
    assert!(tx1["amount"].as_f64().unwrap() < 0.0); // negative (debit)
    let sd1 = tx1["structuredDetails"].as_object().expect("should have structuredDetails");
    assert_eq!(sd1["EREF"], "INV-2026-991");
    assert_eq!(sd1["NAME"], "ALPHA DIGITAL CORP");

    // Second transaction: credit
    let tx2 = &txns[1];
    assert_eq!(tx2["transactionType"], "NTRF");
    assert!(tx2["amount"].as_f64().unwrap() > 0.0); // positive (credit)
    let sd2 = tx2["structuredDetails"].as_object().expect("should have structuredDetails");
    assert_eq!(sd2["EREF"], "TXN-882910");

    // Third transaction: debit, NMSC type
    let tx3 = &txns[2];
    assert_eq!(tx3["transactionType"], "NMSC");
}

#[test]
fn golden_swift_structured_balance_amounts_correct() {
    let raw = fs::read_to_string("tests/data/swift/swift_payload_1.sta").unwrap();
    let chain = DecoderChain::auto();
    let statements = parse_mt940(&raw, &chain).unwrap();
    let stmt = &statements[0];

    assert_eq!(stmt.opening_balance.amount.to_string(), "50000.00");
    assert_eq!(stmt.closing_balance.amount.to_string(), "51500.75");
    assert_eq!(stmt.opening_balance.currency, "EUR");
}

// payload 2: german gvc format

#[test]
fn golden_gvc_parses_two_transactions() {
    let raw = fs::read_to_string("tests/data/gvc/gvc_payload_2.sta").unwrap();
    let chain = DecoderChain::auto();
    let statements = parse_mt940(&raw, &chain).unwrap();

    let stmt = &statements[0];
    assert_eq!(stmt.transactions.len(), 2);
    assert_eq!(stmt.account_identification, "12345678/0009876543");

    // First transaction: GVC 166 (domestic transfer)
    let tx1 = &stmt.transactions[0];
    assert_eq!(tx1.transaction_type, "N166");
    let sd1 = tx1.structured_details.as_ref().unwrap();
    assert_eq!(sd1.get("gvc").unwrap(), "166");
    assert_eq!(sd1.get("32").unwrap(), "ACME CORP GMBH");
    assert_eq!(sd1.get("31").unwrap(), "88776655");

    // Second transaction: GVC 201 (incoming transfer)
    let tx2 = &stmt.transactions[1];
    assert_eq!(tx2.transaction_type, "N201");
    let sd2 = tx2.structured_details.as_ref().unwrap();
    assert_eq!(sd2.get("gvc").unwrap(), "201");
    assert_eq!(sd2.get("32").unwrap(), "MUELLER TRADING CO");

    // GVC balance assertion
    assert_eq!(stmt.opening_balance.amount.to_string(), "120000.00");
    assert_eq!(stmt.closing_balance.amount.to_string(), "132050.00");
}

// payload 3: polish angular format

#[test]
fn golden_angular_parses_polish_format() {
    let raw = fs::read_to_string("tests/data/angular/angular_payload_3.sta").unwrap();
    let chain = DecoderChain::auto();
    let statements = parse_mt940(&raw, &chain).unwrap();

    let stmt = &statements[0];
    assert_eq!(stmt.transactions.len(), 2);
    assert_eq!(stmt.opening_balance.currency, "PLN");

    // First transaction: angular format with < delimiters
    let tx1 = &stmt.transactions[0];
    let sd1 = tx1.structured_details.as_ref().unwrap();
    assert_eq!(sd1.get("tx_code").unwrap(), "010");
    assert_eq!(sd1.get("20").unwrap(), "FAKTURA 1234/2026");
    assert_eq!(sd1.get("27").unwrap(), "JOHN DOE SERVICES");

    // Second transaction
    let tx2 = &stmt.transactions[1];
    let sd2 = tx2.structured_details.as_ref().unwrap();
    assert_eq!(sd2.get("tx_code").unwrap(), "020");
    assert_eq!(sd2.get("27").unwrap(), "ALEXANDRA SMITH SP Z O O");
}

// payload 4: us unstructured format

#[test]
fn golden_unstructured_preserves_raw_text() {
    let raw = fs::read_to_string("tests/data/unstructured/us_payload_4.sta").unwrap();
    let chain = DecoderChain::auto();
    let statements = parse_mt940(&raw, &chain).unwrap();

    let stmt = &statements[0];
    assert_eq!(stmt.transactions.len(), 2);
    assert_eq!(stmt.opening_balance.currency, "USD");

    // Both transactions should have structured_details with "detail" key
    for tx in &stmt.transactions {
        let sd = tx.structured_details.as_ref().unwrap();
        assert!(sd.contains_key("detail"));
        assert!(!sd.get("detail").unwrap().is_empty());
    }

    // Verify raw :86: text is preserved
    let tx1 = &stmt.transactions[0];
    assert!(tx1.details.contains("WIRE TRANSFER OUT TO JOHN DOE"));
    assert!(tx1.details.contains("NEW YORK CORE BRANCH"));
}

// payload 5: stress test (mixed dialects)

#[test]
fn golden_stress_test_detects_mixed_dialects_per_transaction() {
    let raw = fs::read_to_string("tests/data/stress/stress_payload_5.sta").unwrap();
    let chain = DecoderChain::auto();
    let statements = parse_mt940(&raw, &chain).unwrap();

    let stmt = &statements[0];
    assert_eq!(stmt.transactions.len(), 3);

    // Transaction 1: Unknown regional format -> unstructured
    let tx1 = &stmt.transactions[0];
    let sd1 = tx1.structured_details.as_ref().unwrap();
    assert!(sd1.contains_key("detail"));
    assert!(!sd1.contains_key("EREF"));

    // Transaction 2: SWIFT structured with multi-line wrapping
    let tx2 = &stmt.transactions[1];
    let sd2 = tx2.structured_details.as_ref().unwrap();
    assert_eq!(sd2.get("EREF").unwrap(), "STRESS-881");
    assert_eq!(sd2.get("NAME").unwrap(), "ENTERPRISE HOLDINGS PLC");
    // Multi-line concatenation verified: no space injected at line breaks
    assert!(sd2.get("REMI").unwrap().contains("THATSHOULD"));

    // Transaction 3: Unstructured service fee
    let tx3 = &stmt.transactions[2];
    let sd3 = tx3.structured_details.as_ref().unwrap();
    assert!(sd3.contains_key("detail"));
}

#[test]
fn golden_stress_test_multi_line_concatenation_no_space() {
    let raw = fs::read_to_string("tests/data/stress/stress_payload_5.sta").unwrap();
    let chain = DecoderChain::auto();
    let statements = parse_mt940(&raw, &chain).unwrap();

    // Transaction 2 has multi-line :86: with mid-word breaks
    let tx2 = &statements[0].transactions[1];
    let sd = tx2.structured_details.as_ref().unwrap();
    let remi = sd.get("REMI").unwrap();

    // "THAT\nSHOULD" -> "THATSHOULD" (no space)
    assert!(remi.contains("THATSHOULD"));
    // "THE\nMIDDLE" -> "THEMIDDLE" (no space)
    assert!(remi.contains("THEMIDDLE"));
}

// edge cases

#[test]
fn golden_empty_file_returns_error() {
    let result = parse_mt940("", &DecoderChain::auto());
    assert!(result.is_err());
}

#[test]
fn golden_missing_mandatory_tags_returns_error() {
    let raw = ":20:TEST\r\n:62F:C240101EUR100,00\r\n";
    let result = parse_mt940(raw, &DecoderChain::auto());
    assert!(result.is_err());
}

#[test]
fn golden_number_of_statement_handles_number_only() {
    // :28C: without sequence number
    let raw =
        ":20:TEST\r\n:25:ACCT\r\n:28C:00001\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n";
    let statements = parse_mt940(raw, &DecoderChain::auto()).unwrap();
    let stmt = &statements[0];
    assert_eq!(stmt.statement_number.statement, "00001");
    assert!(stmt.statement_number.sequence.is_none());
}
