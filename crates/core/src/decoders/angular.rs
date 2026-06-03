use std::collections::HashMap;

use super::Tag86Decoder;

// AngularDecoder: <DDVALUE<DDVALUE or ^DDVALUE^DDVALUE pattern

/// Decodes Eastern European / Nordic Angular Tag 86 content.
///
/// # Detection
///
/// Matches when the input contains `<` or `^` followed by two numeric
/// digits: the angular subfield pattern used by Polish, Czech, and
/// Nordic banks.
///
/// # Parsing
///
/// First determines the delimiter (`<` or `^`) by checking which appears
/// more frequently in the input. Splits on that delimiter.
/// The first segment before the first delimiter is the transaction type
/// code (stored as `"tx_code"`). Each subsequent segment:
///   - First 2 chars = subfield code
///   - Remaining chars = value
///
/// # Example (Polish)
///
/// ```text
/// Input:  010<00PRZELEW PRZYCHODZACY<20FAKTURA 1234/2026<27JOHN DOE SERVICES<30PL22103004
/// Output: {"tx_code": "010", "00": "PRZELEW PRZYCHODZACY",
///          "20": "FAKTURA 1234/2026", "27": "JOHN DOE SERVICES", "30": "PL22103004"}
/// ```
///
/// # Example (Nordic)
///
/// ```text
/// Input:  099^00INSATTNING^20INVOICE 555^27NORDIC TRADING AB
/// Output: {"tx_code": "099", "00": "INSATTNING", "20": "INVOICE 555",
///          "27": "NORDIC TRADING AB"}
/// ```
pub struct AngularDecoder;

impl Tag86Decoder for AngularDecoder {
    fn name(&self) -> &'static str {
        "angular"
    }

    fn decode(&self, raw: &str) -> Option<HashMap<String, String>> {
        // Determine the delimiter: count occurrences of < vs ^
        let lt_count = raw.chars().filter(|&c| c == '<').count();
        let hat_count = raw.chars().filter(|&c| c == '^').count();

        let delimiter = if lt_count >= hat_count && lt_count > 0 {
            '<'
        } else if hat_count > 0 {
            '^'
        } else {
            return None;
        };

        // Detection: find at least one delimiter followed by 2 digits
        let mut found = false;
        let mut chars = raw.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == delimiter {
                let next_two: String = chars.by_ref().take(2).collect();
                if next_two.len() == 2 && next_two.chars().all(|c| c.is_ascii_digit()) {
                    found = true;
                    break;
                }
            }
        }

        if !found {
            return None;
        }

        let mut fields = HashMap::new();

        let parts: Vec<&str> = raw.split(delimiter).collect();

        // First part before the first delimiter = transaction type code
        if let Some(first) = parts.first() {
            let tx_code = first.trim();
            if !tx_code.is_empty() {
                fields.insert("tx_code".to_string(), tx_code.to_string());
            }
        }

        // Subsequent parts are `DD=VALUE` pairs
        for segment in &parts[1..] {
            if segment.len() < 2 {
                continue;
            }

            let code = &segment[..2];
            let value = segment[2..].trim();

            if code.chars().all(|c| c.is_ascii_digit()) {
                fields.insert(code.to_string(), value.to_string());
            }
        }

        if fields.is_empty() || fields.len() == 1 && fields.contains_key("tx_code") {
            return None;
        }

        Some(fields)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_polish_angular_format() {
        let decoder = AngularDecoder;
        let raw = "010<00PRZELEW PRZYCHODZACY<20FAKTURA 1234/2026<27JOHN DOE SERVICES<30PL22103004";

        let result = decoder.decode(raw);
        assert!(result.is_some());

        let fields = result.unwrap();
        assert_eq!(fields.get("tx_code").unwrap(), "010");
        assert_eq!(fields.get("00").unwrap(), "PRZELEW PRZYCHODZACY");
        assert_eq!(fields.get("20").unwrap(), "FAKTURA 1234/2026");
        assert_eq!(fields.get("27").unwrap(), "JOHN DOE SERVICES");
        assert_eq!(fields.get("30").unwrap(), "PL22103004");
    }

    #[test]
    fn decodes_nordic_caret_format() {
        let decoder = AngularDecoder;
        let raw = "099^00INSATTNING^20INVOICE 555^27NORDIC TRADING AB";

        let result = decoder.decode(raw);
        assert!(result.is_some());

        let fields = result.unwrap();
        assert_eq!(fields.get("tx_code").unwrap(), "099");
        assert_eq!(fields.get("00").unwrap(), "INSATTNING");
        assert_eq!(fields.get("27").unwrap(), "NORDIC TRADING AB");
    }

    #[test]
    fn rejects_straight_text() {
        let decoder = AngularDecoder;
        let result = decoder.decode("WIRE TRANSFER OUT TO JOHN DOE");
        assert!(result.is_none());
    }

    #[test]
    fn rejects_swift_format() {
        let decoder = AngularDecoder;
        let result = decoder.decode("/EREF/INV-001/NAME/CORP");
        assert!(result.is_none());
    }
}
