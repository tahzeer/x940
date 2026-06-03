use std::collections::HashMap;

use super::Tag86Decoder;

// SwiftStructuredDecoder: /KEYWORD/VALUE pattern

/// Decodes SWIFT-structured Tag 86 content using the /KEYWORD/VALUE pattern.
///
/// # Detection
///
/// Matches when the input contains /EREF/, /NAME/, or /REMI/:
/// the three most common SWIFT subfield markers.
///
/// # Parsing
///
/// Regex: /([A-Z]{4})/([^/]+): matches a 4-uppercase-letter keyword
/// followed by a value delimited by the next slash.
///
/// # Example
///
/// ```text
/// Input:  /EREF/INV-2026-991/REMI/MONTHLY RETAINER FEES/NAME/ALPHA DIGITAL CORP
/// Output: {"EREF": "INV-2026-991", "REMI": "MONTHLY RETAINER FEES", "NAME": "ALPHA DIGITAL CORP"}
/// ```
pub struct SwiftStructuredDecoder;

impl Tag86Decoder for SwiftStructuredDecoder {
    fn name(&self) -> &'static str {
        "swift-structured"
    }

    fn decode(&self, raw: &str) -> Option<HashMap<String, String>> {
        // Detection: must contain at least one known SWIFT subfield marker
        if !raw.contains("/EREF/")
            && !raw.contains("/NAME/")
            && !raw.contains("/REMI/")
            && !raw.contains("/BIC/")
            && !raw.contains("/IBAN/")
        {
            return None;
        }

        let mut fields = HashMap::new();

        // Split on "/": the first segment is empty (leading slash) or
        // non-keyword prefix text; skip it
        let segments: Vec<&str> = raw.split('/').collect();

        let mut i = 0;
        while i + 1 < segments.len() {
            let keyword = segments[i];
            let value = segments[i + 1];

            // SWIFT subfield keywords are 3-4 uppercase ASCII letters
            if keyword.len() >= 3
                && keyword.len() <= 4
                && keyword.chars().all(|c| c.is_ascii_uppercase())
            {
                fields.insert(keyword.to_string(), value.trim().to_string());
            }

            i += 1;
        }

        if fields.is_empty() {
            return None;
        }

        Some(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_standard_swift_format() {
        let decoder = SwiftStructuredDecoder;
        let raw = "/EREF/INV-2026-991/REMI/MONTHLY RETAINER/NAME/ALPHA CORP";

        let result = decoder.decode(raw);
        assert!(result.is_some());

        let fields = result.unwrap();
        assert_eq!(fields.get("EREF").unwrap(), "INV-2026-991");
        assert_eq!(fields.get("REMI").unwrap(), "MONTHLY RETAINER");
        assert_eq!(fields.get("NAME").unwrap(), "ALPHA CORP");
    }

    #[test]
    fn rejects_non_swift_input() {
        let decoder = SwiftStructuredDecoder;
        let result = decoder.decode("UNKNOWN REGIONAL FORMAT CODE");
        assert!(result.is_none());
    }

    #[test]
    fn rejects_german_gvc_format() {
        let decoder = SwiftStructuredDecoder;
        let result = decoder.decode("166?00REMITTANCE?20INV-9924?32ACME CORP");
        assert!(result.is_none());
    }
}
