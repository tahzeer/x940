//!
//! Tag 86 dialect decoders.
//!
//! Each decoder implements the Tag86Decoder trait and attempts to
//! parse a raw :86: text block into structured key-value pairs.
//!
//! Decoders run **per-transaction**: a single MT940 statement can
//! contain transactions from multiple dialects. The DecoderChain
//! tries each decoder in sequence; the first decoder that returns
//! Some(...) wins for that specific transaction.
//!
//! ## Decoder Chain (auto-detect order)
//!
//! 1. SwiftStructuredDecoder: /KEYWORD/VALUE pattern
//! 2. GermanGvcDecoder: GVC?DDVALUE?DDVALUE pattern
//! 3. AngularDecoder: <DDVALUE<DDVALUE or ^DDVALUE^DDVALUE pattern
//! 4. UnstructuredDecoder: always matches (safety net, last in chain)

use std::collections::HashMap;

pub mod angular;
pub mod german_gvc;
pub mod swift_structured;
pub mod unstructured;

pub use self::angular::AngularDecoder;
pub use self::german_gvc::GermanGvcDecoder;
pub use self::swift_structured::SwiftStructuredDecoder;
pub use self::unstructured::UnstructuredDecoder;

/// Trait for Tag 86 narrative decoders.
///
/// Each dialect implements this trait with its own parsing logic.
/// A decoder that doesn't recognize the input returns None,
/// allowing the chain to try the next decoder.
pub trait Tag86Decoder: Send + Sync {
    /// Attempt to parse a raw Tag 86 block into structured key-value pairs.
    ///
    /// Returns `None` if this decoder doesn't recognize the format,
    /// allowing fallback to the next decoder in the chain.
    fn decode(&self, raw: &str) -> Option<HashMap<String, String>>;

    /// Human-readable name for logging and debugging.
    fn name(&self) -> &'static str;
}

/// A chain of Tag86Decoder implementations tried in order.
///
/// The chain is configurable: auto() uses all four decoders,
/// with_resolver() prioritizes a specific dialect while keeping
/// the unstructured fallback as a safety net.
pub struct DecoderChain {
    decoders: Vec<Box<dyn Tag86Decoder>>,
}

impl DecoderChain {
    /// Full auto-detect chain: all four decoders in priority order.
    ///
    /// ```
    /// use x940rs::DecoderChain;
    /// let chain = DecoderChain::auto();
    /// ```
    pub fn auto() -> Self {
        Self {
            decoders: vec![
                Box::new(SwiftStructuredDecoder),
                Box::new(GermanGvcDecoder),
                Box::new(AngularDecoder),
                Box::new(UnstructuredDecoder), // always matches: safety net
            ],
        }
    }

    /// Explicit resolver: chosen decoder + unstructured safety net.
    ///
    /// When a specific dialect is requested, the chain contains only
    /// the chosen decoder followed by `UnstructuredDecoder` as fallback.
    /// Transactions that don't match the chosen dialect fall through
    /// safely rather than causing a parse failure.
    ///
    /// Resolver values (unified across all bindings):
    ///   - "swift": SWIFT /KEYWORD/ structured format
    ///   - "gvc": German ?DD GVC format
    ///   - "angular": Polish/Nordic <DD or ^DD format
    ///   - "auto": all four decoders (equivalent to auto())
    ///
    /// ```
    /// use x940rs::DecoderChain;
    ///
    /// let chain = DecoderChain::with_resolver("gvc").unwrap();
    /// let chain = DecoderChain::with_resolver("swift").unwrap();
    /// let chain = DecoderChain::with_resolver("angular").unwrap();
    /// // "auto" is equivalent to DecoderChain::auto()
    /// let chain = DecoderChain::with_resolver("auto").unwrap();
    /// ```
    pub fn with_resolver(resolver: &str) -> Option<Self> {
        let primary: Box<dyn Tag86Decoder> = match resolver {
            "swift" => Box::new(SwiftStructuredDecoder),
            "gvc" => Box::new(GermanGvcDecoder),
            "angular" => Box::new(AngularDecoder),
            "auto" => return Some(Self::auto()),
            _ => return None,
        };

        Some(Self {
            decoders: vec![primary, Box::new(UnstructuredDecoder)],
        })
    }

    /// Decode a raw Tag 86 text block.
    ///
    /// Returns the parsed key-value pairs from the first decoder that
    /// matches, or the raw text under the `"detail"` key from
    /// `UnstructuredDecoder` (which always matches as the last decoder).
    pub fn decode(&self, raw: &str) -> HashMap<String, String> {
        for decoder in &self.decoders {
            if let Some(result) = decoder.decode(raw) {
                return result;
            }
        }
        // Unreachable: UnstructuredDecoder is always last and always matches
        unreachable!("UnstructuredDecoder must be the last decoder in every chain");
    }
}

// Calculator Chain for convenience properties

/// Compute the counterparty name from structured details.
///
/// Resolution order:
///   1. GVC `?32` + `?33` (space-joined)
///   2. Angular `27`
///   3. SWIFT `NAME`
#[allow(dead_code)]
pub(crate) fn resolve_counterparty(sd: &HashMap<String, String>) -> String {
    if let Some(name) = sd.get("32") {
        if let Some(cont) = sd.get("33") {
            return format!("{} {}", name, cont);
        }
        return name.clone();
    }
    if let Some(name) = sd.get("27") {
        return name.clone();
    }
    sd.get("NAME").cloned().unwrap_or_default()
}

/// Compute the counterparty IBAN from structured details.
///
/// Resolution order:
///   1. GVC `?31` (Kontonummer)
///   2. Angular `30`
///   3. SWIFT `IBAN`
#[allow(dead_code)]
pub(crate) fn resolve_counter_iban(sd: &HashMap<String, String>) -> String {
    sd.get("31").or_else(|| sd.get("30")).or_else(|| sd.get("IBAN")).cloned().unwrap_or_default()
}

/// Compute the purpose/remittance text from structured details.
///
/// Resolution order:
///   1. GVC `?20` through `?29` joined with spaces
///   2. SWIFT `REMI`
///   3. SWIFT `EREF`
#[allow(dead_code)]
pub(crate) fn resolve_purpose(sd: &HashMap<String, String>) -> String {
    // GVC concatenation: ?20 through ?29 joined with spaces
    let gvc_lines: Vec<String> =
        (20..=29).filter_map(|i| sd.get(&i.to_string())).cloned().collect();

    if !gvc_lines.is_empty() {
        return gvc_lines.join(" ");
    }

    sd.get("REMI").or_else(|| sd.get("EREF")).cloned().unwrap_or_default()
}
