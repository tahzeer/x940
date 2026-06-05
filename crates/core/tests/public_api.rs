//!
//! Integration tests: public API surface and dialect auto-detection.
//!
//! These tests verify the public API entry points work correctly
//! without needing the full FSM parser implementation.

use x940rs::{Balance, DebitOrCredit, DecoderChain, Statement, StatementNumber, Transaction};

// public API type availability

#[test]
fn public_types_are_available() {
    // Verify all key types are accessible from the public API
    let chain = DecoderChain::auto();
    let _output = chain.decode("test");

    // DebitOrCredit enum variants
    assert_eq!(DebitOrCredit::Debit.to_string(), "D");
    assert_eq!(DebitOrCredit::Credit.to_string(), "C");
    assert_eq!(DebitOrCredit::ReversalDebit.to_string(), "RD");
    assert_eq!(DebitOrCredit::ReversalCredit.to_string(), "RC");
}

#[test]
fn debit_or_credit_is_credit_detection() {
    assert!(!DebitOrCredit::Debit.is_credit());
    assert!(DebitOrCredit::Credit.is_credit());
    assert!(DebitOrCredit::ReversalDebit.is_credit());
    assert!(!DebitOrCredit::ReversalCredit.is_credit());
}

#[test]
fn debit_or_credit_is_debit_detection() {
    assert!(DebitOrCredit::Debit.is_debit());
    assert!(!DebitOrCredit::Credit.is_debit());
    assert!(!DebitOrCredit::ReversalDebit.is_debit());
    assert!(DebitOrCredit::ReversalCredit.is_debit());
}

#[test]
fn debit_or_credit_is_reversal_detection() {
    assert!(!DebitOrCredit::Debit.is_reversal());
    assert!(!DebitOrCredit::Credit.is_reversal());
    assert!(DebitOrCredit::ReversalDebit.is_reversal());
    assert!(DebitOrCredit::ReversalCredit.is_reversal());
}

#[test]
fn debit_or_credit_effective_maps_reversals() {
    // ReversalDebit effective is Credit
    assert_eq!(DebitOrCredit::ReversalDebit.effective(), DebitOrCredit::Credit);
    // ReversalCredit effective is Debit
    assert_eq!(DebitOrCredit::ReversalCredit.effective(), DebitOrCredit::Debit);
    // Non-reversals return themselves
    assert_eq!(DebitOrCredit::Debit.effective(), DebitOrCredit::Debit);
    assert_eq!(DebitOrCredit::Credit.effective(), DebitOrCredit::Credit);
}

// decoder chain public API

#[test]
fn decoder_chain_auto_returns_non_empty_result() {
    let chain = DecoderChain::auto();
    let result = chain.decode("ANY TEXT");
    assert!(!result.is_empty());
}

#[test]
fn decoder_chain_decode_always_returns_map() {
    let chain = DecoderChain::auto();

    // Various inputs should all produce a non-empty HashMap
    assert!(!chain.decode("").is_empty());
    assert!(!chain.decode(" ").is_empty());
    assert!(!chain.decode("TEST").is_empty());
}

#[test]
fn decoder_chain_with_resolver_rejects_invalid() {
    assert!(DecoderChain::with_resolver("invalid").is_none());
}

#[test]
fn decoder_chain_with_resolver_accepts_valid() {
    assert!(DecoderChain::with_resolver("swift").is_some());
    assert!(DecoderChain::with_resolver("gvc").is_some());
    assert!(DecoderChain::with_resolver("angular").is_some());
    assert!(DecoderChain::with_resolver("auto").is_some());
}

// transaction amount signing

#[test]
fn transaction_signed_amount_positive_for_credit() {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let tx = Transaction {
        value_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        entry_date: None,
        debit_credit: DebitOrCredit::Credit,
        funds_code: None,
        amount: Decimal::from_str("1500.00").unwrap(),
        transaction_type: "NTRF".into(),
        customer_reference: "".into(),
        bank_reference: None,
        supplementary: None,
        details: "".into(),
        structured_details: None,
    };

    assert_eq!(tx.signed_amount(), Decimal::from_str("1500.00").unwrap());
}

#[test]
fn transaction_signed_amount_negative_for_debit() {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let tx = Transaction {
        value_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        entry_date: None,
        debit_credit: DebitOrCredit::Debit,
        funds_code: None,
        amount: Decimal::from_str("1500.00").unwrap(),
        transaction_type: "NTRF".into(),
        customer_reference: "".into(),
        bank_reference: None,
        supplementary: None,
        details: "".into(),
        structured_details: None,
    };

    assert_eq!(tx.signed_amount(), Decimal::from_str("-1500.00").unwrap());
}

#[test]
fn transaction_signed_amount_positive_for_reversal_debit() {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let tx = Transaction {
        value_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        entry_date: None,
        debit_credit: DebitOrCredit::ReversalDebit,
        funds_code: None,
        amount: Decimal::from_str("500.00").unwrap(),
        transaction_type: "NTRF".into(),
        customer_reference: "".into(),
        bank_reference: None,
        supplementary: None,
        details: "".into(),
        structured_details: None,
    };

    // ReversalDebit -> treated as credit -> positive
    assert_eq!(tx.signed_amount(), Decimal::from_str("500.00").unwrap());
}

#[test]
fn transaction_signed_amount_negative_for_reversal_credit() {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let tx = Transaction {
        value_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        entry_date: None,
        debit_credit: DebitOrCredit::ReversalCredit,
        funds_code: None,
        amount: Decimal::from_str("500.00").unwrap(),
        transaction_type: "NTRF".into(),
        customer_reference: "".into(),
        bank_reference: None,
        supplementary: None,
        details: "".into(),
        structured_details: None,
    };

    // ReversalCredit -> treated as debit -> negative
    assert_eq!(tx.signed_amount(), Decimal::from_str("-500.00").unwrap());
}

// counterparty resolution (via structured_details)

#[test]
fn transaction_counterparty_from_swift_name() {
    use rust_decimal::Decimal;
    use std::collections::HashMap;
    use std::str::FromStr;

    let mut sd = HashMap::new();
    sd.insert("NAME".into(), "ALPHA DIGITAL CORP".into());

    let tx = Transaction {
        value_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        entry_date: None,
        debit_credit: DebitOrCredit::Debit,
        funds_code: None,
        amount: Decimal::from_str("100.00").unwrap(),
        transaction_type: "NTRF".into(),
        customer_reference: "".into(),
        bank_reference: None,
        supplementary: None,
        details: "".into(),
        structured_details: Some(sd),
    };

    assert_eq!(tx.counterparty().unwrap(), "ALPHA DIGITAL CORP");
}

#[test]
fn transaction_counterparty_from_gvc_32() {
    use rust_decimal::Decimal;
    use std::collections::HashMap;
    use std::str::FromStr;

    let mut sd = HashMap::new();
    sd.insert("32".into(), "ACME CORP GMBH".into());

    let tx = Transaction {
        value_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        entry_date: None,
        debit_credit: DebitOrCredit::Debit,
        funds_code: None,
        amount: Decimal::from_str("100.00").unwrap(),
        transaction_type: "NTRF".into(),
        customer_reference: "".into(),
        bank_reference: None,
        supplementary: None,
        details: "".into(),
        structured_details: Some(sd),
    };

    assert_eq!(tx.counterparty().unwrap(), "ACME CORP GMBH");
}

#[test]
fn transaction_counterparty_from_angular_27() {
    use rust_decimal::Decimal;
    use std::collections::HashMap;
    use std::str::FromStr;

    let mut sd = HashMap::new();
    sd.insert("27".into(), "JOHN DOE SERVICES".into());

    let tx = Transaction {
        value_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        entry_date: None,
        debit_credit: DebitOrCredit::Debit,
        funds_code: None,
        amount: Decimal::from_str("100.00").unwrap(),
        transaction_type: "NTRF".into(),
        customer_reference: "".into(),
        bank_reference: None,
        supplementary: None,
        details: "".into(),
        structured_details: Some(sd),
    };

    assert_eq!(tx.counterparty().unwrap(), "JOHN DOE SERVICES");
}

#[test]
fn transaction_counterparty_returns_none_for_unstructured() {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let tx = Transaction {
        value_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        entry_date: None,
        debit_credit: DebitOrCredit::Debit,
        funds_code: None,
        amount: Decimal::from_str("100.00").unwrap(),
        transaction_type: "NTRF".into(),
        customer_reference: "".into(),
        bank_reference: None,
        supplementary: None,
        details: "WIRE TRANSFER".into(),
        structured_details: None,
    };

    assert!(tx.counterparty().is_none());
}

// statement currency and date accessors

#[test]
fn statement_currency_returns_opening_balance_currency() {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let stmt = Statement {
        transaction_reference: "TEST".into(),
        related_reference: None,
        account_identification: "ACCT".into(),
        statement_number: StatementNumber {
            statement: "00001".into(),
            sequence: None,
        },
        opening_balance: Balance {
            is_intermediate: false,
            debit_credit: DebitOrCredit::Credit,
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            currency: "EUR".into(),
            amount: Decimal::from_str("1000.00").unwrap(),
        },
        closing_balance: Balance {
            is_intermediate: false,
            debit_credit: DebitOrCredit::Credit,
            date: NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            currency: "EUR".into(),
            amount: Decimal::from_str("1000.00").unwrap(),
        },
        closing_available: None,
        forward_available: None,
        transactions: vec![],
        info_to_owner: None,
        has_opening_balance: true,
        has_closing_balance: true,
    };

    assert_eq!(stmt.currency(), "EUR");
    assert_eq!(stmt.statement_date(), NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
}
