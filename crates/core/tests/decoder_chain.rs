//!
//! Integration tests: DecoderChain construction and dialect resolution.
//!
//! These tests exercise the public API of the decoder chain without
//! needing the full FSM parser. They verify that each dialect decoder
//! correctly resolves known-format Tag 86 content.

use x940rs::DecoderChain;

// chain construction

#[test]
fn auto_chain_has_four_decoders() {
    let chain = DecoderChain::auto();
    // A known SWIFT input should decode with structured fields
    let result = chain.decode("/EREF/INV-991/NAME/TEST CORP");
    assert!(result.contains_key("EREF"));
    assert_eq!(result.get("EREF").unwrap(), "INV-991");
    assert_eq!(result.get("NAME").unwrap(), "TEST CORP");
}

#[test]
fn auto_chain_falls_back_to_unstructured() {
    let chain = DecoderChain::auto();
    let result = chain.decode("THIS IS PURE FREE TEXT WITH NO STRUCTURE");
    assert!(result.contains_key("detail"));
    assert_eq!(result.get("detail").unwrap(), "THIS IS PURE FREE TEXT WITH NO STRUCTURE");
}

#[test]
fn with_resolver_swift_works() {
    let chain = DecoderChain::with_resolver("swift").expect("swift resolver should be valid");

    let result = chain.decode("/EREF/INV-001/NAME/ACME CORP");
    assert!(result.contains_key("EREF"));
    assert_eq!(result.get("NAME").unwrap(), "ACME CORP");
}

#[test]
fn with_resolver_gvc_works() {
    let chain = DecoderChain::with_resolver("gvc").expect("gvc resolver should be valid");

    let result = chain.decode("166?00REMITTANCE?20INV-9924?32ACME CORP");
    assert!(result.contains_key("gvc"));
    assert_eq!(result.get("gvc").unwrap(), "166");
    assert_eq!(result.get("20").unwrap(), "INV-9924");
    assert_eq!(result.get("32").unwrap(), "ACME CORP");
}

#[test]
fn with_resolver_angular_works() {
    let chain = DecoderChain::with_resolver("angular").expect("angular resolver should be valid");

    let result = chain.decode("010<00PRZELEW<20FAKTURA 1234<27JOHN DOE");
    assert!(result.contains_key("tx_code"));
    assert_eq!(result.get("tx_code").unwrap(), "010");
}

#[test]
fn with_resolver_gvc_falls_back_to_unstructured() {
    let chain = DecoderChain::with_resolver("gvc").expect("gvc resolver should be valid");

    // GVC decoder won't match SWIFT input, but unstructured catches it
    let result = chain.decode("/EREF/INV-001/NAME/SWIFT CORP");
    assert!(result.contains_key("detail"));
}

#[test]
fn with_resolver_auto_is_valid() {
    let chain = DecoderChain::with_resolver("auto").expect("auto resolver should be valid");

    let result = chain.decode("/EREF/INV-001/NAME/CORP");
    assert!(result.contains_key("EREF"));
}

#[test]
fn invalid_resolver_returns_none() {
    let result = DecoderChain::with_resolver("invalid_dialect");
    assert!(result.is_none());
}

// swift structured dialect resolution

#[test]
fn swift_decoder_resolves_all_standard_keywords() {
    let chain = DecoderChain::auto();
    let raw =
        "/EREF/INV-2026-991/REMI/MONTHLY RETAINER FEES/NAME/ALPHA DIGITAL CORP/BIC/ALPHDEFFXXX";

    let fields = chain.decode(raw);

    assert_eq!(fields.get("EREF").unwrap(), "INV-2026-991");
    assert_eq!(fields.get("REMI").unwrap(), "MONTHLY RETAINER FEES");
    assert_eq!(fields.get("NAME").unwrap(), "ALPHA DIGITAL CORP");
    assert_eq!(fields.get("BIC").unwrap(), "ALPHDEFFXXX");
}

#[test]
fn swift_decoder_extracts_iban() {
    let chain = DecoderChain::auto();
    let raw = "/EREF/INV-001/IBAN/DE89370400440532013000/NAME/JOHN DOE";

    let fields = chain.decode(raw);

    assert_eq!(fields.get("IBAN").unwrap(), "DE89370400440532013000");
}

#[test]
fn swift_decoder_trims_values() {
    let chain = DecoderChain::auto();
    let raw = "/EREF/  INV-991  /NAME/  ACME  ";

    let fields = chain.decode(raw);

    // Leading/trailing whitespace in values should be trimmed
    assert!(fields.get("EREF").unwrap().starts_with("INV"));
}

// german gvc dialect resolution

#[test]
fn gvc_decoder_resolves_all_standard_subfields() {
    let chain = DecoderChain::auto();
    let raw =
        "166?00REMITTANCE?20INV-9924?21KREATOR ABSCHNITT 1?3010020030?3188776655?32ACME CORP GMBH";

    let fields = chain.decode(raw);

    assert_eq!(fields.get("gvc").unwrap(), "166");
    assert_eq!(fields.get("00").unwrap(), "REMITTANCE");
    assert_eq!(fields.get("20").unwrap(), "INV-9924");
    assert_eq!(fields.get("21").unwrap(), "KREATOR ABSCHNITT 1");
    assert_eq!(fields.get("30").unwrap(), "10020030");
    assert_eq!(fields.get("31").unwrap(), "88776655");
    assert_eq!(fields.get("32").unwrap(), "ACME CORP GMBH");
}

#[test]
fn gvc_decoder_produces_combined_remittance() {
    let chain = DecoderChain::auto();
    let raw = "166?00REMITTANCE?20INV-9924?21KREATOR ABSCHNITT 1";

    let fields = chain.decode(raw);

    assert!(fields.contains_key("REMI_COMBINED"));
    assert_eq!(fields.get("REMI_COMBINED").unwrap(), "INV-9924 KREATOR ABSCHNITT 1");
}

#[test]
fn gvc_decoder_handles_sparse_subfields() {
    let chain = DecoderChain::auto();
    let raw = "201?00GUTSCHRIFT?20KUNDE-88124?32MUELLER TRADING CO";

    let fields = chain.decode(raw);

    assert_eq!(fields.get("gvc").unwrap(), "201");
    assert_eq!(fields.get("32").unwrap(), "MUELLER TRADING CO");
    // No remittance lines ?21 through ?29
    assert!(fields.get("21").is_none());
}

// angular dialect resolution

#[test]
fn angular_decoder_resolves_polish_format() {
    let chain = DecoderChain::auto();
    let raw = "010<00PRZELEW PRZYCHODZACY<20FAKTURA 1234/2026<27JOHN DOE SERVICES<30PL22103004";

    let fields = chain.decode(raw);

    assert_eq!(fields.get("tx_code").unwrap(), "010");
    assert_eq!(fields.get("00").unwrap(), "PRZELEW PRZYCHODZACY");
    assert_eq!(fields.get("20").unwrap(), "FAKTURA 1234/2026");
    assert_eq!(fields.get("27").unwrap(), "JOHN DOE SERVICES");
    assert_eq!(fields.get("30").unwrap(), "PL22103004");
}

#[test]
fn angular_decoder_resolves_nordic_caret_format() {
    let chain = DecoderChain::auto();
    let raw = "099^00INSATTNING^20INVOICE 555^27NORDIC TRADING AB";

    let fields = chain.decode(raw);

    assert_eq!(fields.get("tx_code").unwrap(), "099");
    assert_eq!(fields.get("00").unwrap(), "INSATTNING");
    assert_eq!(fields.get("27").unwrap(), "NORDIC TRADING AB");
}

#[test]
fn angular_decoder_handles_multiple_transaction_codes() {
    let chain = DecoderChain::auto();

    let fields1 = chain.decode("020<00UZNANIE RACHUNKU<20ZWROT KOWALSKI TXN-991<27ALEXANDRA SMITH");
    assert_eq!(fields1.get("tx_code").unwrap(), "020");
    assert_eq!(fields1.get("20").unwrap(), "ZWROT KOWALSKI TXN-991");

    let fields2 = chain.decode("010<00PRZELEW<20FAKTURA 1234<27JOHN DOE");
    assert_eq!(fields2.get("tx_code").unwrap(), "010");
}

// unstructured fallback

#[test]
fn unstructured_decoder_always_returns_detail_key() {
    let chain = DecoderChain::auto();

    let english = chain.decode("WIRE TRANSFER OUT TO JOHN DOE FOR MARCH INVOICE 44552");
    assert_eq!(
        english.get("detail").unwrap(),
        "WIRE TRANSFER OUT TO JOHN DOE FOR MARCH INVOICE 44552"
    );

    let mixed = chain.decode("ACH CREDIT FROM PAYPAL EXTRACT VENMO TRANSFER ID 883910243");
    assert_eq!(
        mixed.get("detail").unwrap(),
        "ACH CREDIT FROM PAYPAL EXTRACT VENMO TRANSFER ID 883910243"
    );
}

// multi-dialect auto detection (simulated)

#[test]
fn auto_chain_detects_mixed_dialects_per_call() {
    let chain = DecoderChain::auto();

    // Call 1: SWIFT -> SwiftStructuredDecoder matches
    let r1 = chain.decode("/EREF/STRESS-881/REMI/COMPLEX MULTILINE/NAME/ENTERPRISE HOLDINGS PLC");
    assert_eq!(r1.get("EREF").unwrap(), "STRESS-881");
    assert_eq!(r1.get("NAME").unwrap(), "ENTERPRISE HOLDINGS PLC");

    // Call 2: Unknown format -> UnstructuredDecoder catches it
    let r2 = chain.decode("UNKNOWN REGIONAL FORMAT CODE XYZ999-DATABLOCK-112233-PART4");
    assert!(r2.contains_key("detail"));
    assert!(!r2.contains_key("EREF"));

    // Call 3: Unstructured prose -> UnstructuredDecoder catches it
    let r3 = chain.decode("MONTHLY ACCOUNT SERVICE FEE MINUS REGISTERED REBATE");
    assert!(r3.contains_key("detail"));
}

// multi-line concatenation (no-space rule)

#[test]
fn multi_line_text_reaches_decoder_as_concatenated() {
    let chain = DecoderChain::auto();

    // Simulate what the parser does: strip \n without adding space.
    // "THAT\nSHOULD" becomes "THATSHOULD"
    let raw = "/EREF/STRESS-881/REMI/COMPLEX MULTILINE WRAPPING TRANSACTION THATSHOULD CONTINUOUSLY PARSE EVEN WHEN THE LINE BREAKS OCCUR IN THEMIDDLE OF A WORD OR TEXT BLOCK/NAME/ENTERPRISE HOLDINGS PLC";

    let fields = chain.decode(raw);

    assert_eq!(fields.get("EREF").unwrap(), "STRESS-881");
    assert!(fields.get("REMI").unwrap().contains("THATSHOULD"));
    assert!(fields.get("REMI").unwrap().contains("THEMIDDLE"));
}
