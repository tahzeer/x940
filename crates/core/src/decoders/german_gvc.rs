use std::collections::HashMap;

use super::Tag86Decoder;

// GermanGvcDecoder: GVC?DDVALUE?DDVALUE pattern

/// Decodes German GVC (Geschaftsvorfallcode) Tag 86 content.
///
/// # Detection
///
/// Matches when the input contains a `?` character followed by two numeric
/// digits: the ?DD subfield pattern from the ZKA standard.
///
/// # Parsing
///
/// Splits on `?`. The first segment before the first `?` is the GVC code
/// (3 digits). Each subsequent segment is a `DD=VALUE` pair:
///   - First 2 chars = subfield code (00-63)
///   - Remaining chars = value for that code
///
/// Subfields `?20` through `?29` are concatenated with a space
/// and stored under the key `"REMI_COMBINED"` for convenience.
///
/// # Example
///
/// ```text
/// Input:  166?00REMITTANCE?20INV-9924?21KREATOR ABSCHNITT 1?32ACME CORP GMBH
/// Output: {"gvc": "166", "00": "REMITTANCE", "20": "INV-9924",
///          "21": "KREATOR ABSCHNITT 1", "32": "ACME CORP GMBH",
///          "REMI_COMBINED": "INV-9924 KREATOR ABSCHNITT 1"}
/// ```
pub struct GermanGvcDecoder;

impl Tag86Decoder for GermanGvcDecoder {
    fn name(&self) -> &'static str {
        "german-gvc"
    }

    fn decode(&self, raw: &str) -> Option<HashMap<String, String>> {
        // Detection: must contain "?" with two numeric digits following
        let has_gvc_pattern = raw
            .split('?')
            .skip(1)
            .any(|seg| seg.len() >= 2 && seg[..2].chars().all(|c| c.is_ascii_digit()));

        if !has_gvc_pattern {
            return None;
        }

        let mut fields = HashMap::new();

        let parts: Vec<&str> = raw.split('?').collect();

        // First part is the GVC code
        if let Some(first) = parts.first() {
            let gvc_code: String = first.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !gvc_code.is_empty() {
                fields.insert("gvc".to_string(), gvc_code);
            }
        }

        // Subsequent parts are `DD=VALUE` pairs
        let mut remi_lines: Vec<String> = Vec::new();

        for segment in &parts[1..] {
            if segment.len() < 2 {
                continue;
            }

            let code = &segment[..2];
            let value = segment[2..].trim();

            if code.chars().all(|c| c.is_ascii_digit()) {
                fields.insert(code.to_string(), value.to_string());

                // Accumulate remittance lines (?20 through ?29)
                if let Ok(n) = code.parse::<u32>() {
                    if (20..=29).contains(&n) {
                        remi_lines.push(value.to_string());
                    }
                }
            }
        }

        if fields.is_empty() {
            return None;
        }

        // Add combined remittance for convenience
        if !remi_lines.is_empty() {
            fields.insert("REMI_COMBINED".to_string(), remi_lines.join(" "));
        }

        Some(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_standard_gvc_format() {
        let decoder = GermanGvcDecoder;
        let raw = "166?00REMITTANCE?20INV-9924?21KREATOR ABSCHNITT 1?32ACME CORP GMBH";

        let result = decoder.decode(raw);
        assert!(result.is_some());

        let fields = result.unwrap();
        assert_eq!(fields.get("gvc").unwrap(), "166");
        assert_eq!(fields.get("00").unwrap(), "REMITTANCE");
        assert_eq!(fields.get("20").unwrap(), "INV-9924");
        assert_eq!(fields.get("21").unwrap(), "KREATOR ABSCHNITT 1");
        assert_eq!(fields.get("32").unwrap(), "ACME CORP GMBH");
        assert_eq!(fields.get("REMI_COMBINED").unwrap(), "INV-9924 KREATOR ABSCHNITT 1");
    }

    #[test]
    fn rejects_non_gvc_input() {
        let decoder = GermanGvcDecoder;
        let result = decoder.decode("/EREF/INV-001/NAME/SWIFT CORP");
        assert!(result.is_none());
    }

    #[test]
    fn rejects_angular_format() {
        let decoder = GermanGvcDecoder;
        let result = decoder.decode("010<00PRZELEW<20FAKTURA 1234<27JOHN DOE");
        assert!(result.is_none());
    }
}
