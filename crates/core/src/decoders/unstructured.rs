use std::collections::HashMap;

use super::Tag86Decoder;

// UnstructuredDecoder: always-matches fallback

/// Captures raw free-text Tag 86 content as a single "detail" key.
///
/// This decoder is always the last in every [`DecoderChain`](super::DecoderChain).
/// It never returns None: when no structured dialect decoder matches,
/// this decoder preserves the raw text so no data is ever lost.
///
/// # Example
///
/// ```text
/// Input:  WIRE TRANSFER OUT TO JOHN DOE FOR MARCH INVOICE 44552
/// Output: {"detail": "WIRE TRANSFER OUT TO JOHN DOE FOR MARCH INVOICE 44552"}
/// ```
pub struct UnstructuredDecoder;

impl Tag86Decoder for UnstructuredDecoder {
    fn name(&self) -> &'static str {
        "unstructured"
    }

    fn decode(&self, raw: &str) -> Option<HashMap<String, String>> {
        let mut fields = HashMap::new();
        fields.insert("detail".to_string(), raw.to_string());
        Some(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn always_matches() {
        let decoder = UnstructuredDecoder;
        let result = decoder.decode("ANY RANDOM TEXT HERE");
        assert!(result.is_some());
        let fields = result.unwrap();
        assert_eq!(fields.get("detail").unwrap(), "ANY RANDOM TEXT HERE");
    }

    #[test]
    fn handles_multi_line_text() {
        let decoder = UnstructuredDecoder;
        let raw = "LINE ONE\nLINE TWO\nLINE THREE";
        let result = decoder.decode(raw);
        assert!(result.is_some());
        let fields = result.unwrap();
        assert_eq!(fields.len(), 1);
    }
}
