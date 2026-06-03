use rust_decimal::Decimal;

use crate::error::ParseError;

/// Parse an MT940 amount string to `Decimal`.
///
/// MT940 uses **comma** as the decimal separator (European convention).
///
/// # Format
///
/// integer_part,decimal_part: decimal_part is 0-2 digits.
///
/// # Examples
///
/// ```text
/// "50000,00" → Decimal(5000000, 2)   = 50000.00
/// "1250,5"   → Decimal(125050, 2)    = 1250.50  (trailing zero implied)
/// "100,"     → Decimal(10000, 2)     = 100.00   (empty decimal → .00)
/// "0,12"     → Decimal(12, 2)        = 0.12
/// ```
#[allow(dead_code)]
pub fn parse_mt940_amount(raw: &str) -> Result<Decimal, ParseError> {
    // Normalize: replace comma with dot for Rust's Decimal parser
    let normalized = raw.replace(",", ".");

    let parts: Vec<&str> = normalized.split('.').collect();
    let integer = parts[0].trim_start_matches('0');
    let integer = if integer.is_empty() { "0" } else { integer };
    let decimal = if parts.len() > 1 { parts[1] } else { "" };

    // Pad decimal to exactly 2 places
    let decimal_padded = format!("{:0<2}", decimal);

    let combined = format!("{}.{}", integer, decimal_padded);

    combined.parse::<Decimal>().map_err(|_| ParseError::InvalidAmount {
        value: raw.to_string(),
        tag: "(amount field)",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    #[test]
    fn parses_standard_amount() {
        let result = parse_mt940_amount("50000,00").unwrap();
        assert_eq!(result, Decimal::from_str("50000.00").unwrap());
    }

    #[test]
    fn parses_amount_with_single_decimal() {
        let result = parse_mt940_amount("1250,5").unwrap();
        assert_eq!(result, Decimal::from_str("1250.50").unwrap());
    }

    #[test]
    fn parses_amount_with_empty_decimal() {
        let result = parse_mt940_amount("100,").unwrap();
        assert_eq!(result, Decimal::from_str("100.00").unwrap());
    }

    #[test]
    fn parses_small_amount() {
        let result = parse_mt940_amount("0,12").unwrap();
        assert_eq!(result, Decimal::from_str("0.12").unwrap());
    }

    #[test]
    fn parses_integer_only() {
        let result = parse_mt940_amount("100").unwrap();
        assert_eq!(result, Decimal::from_str("100.00").unwrap());
    }

    #[test]
    fn preserves_two_decimal_places() {
        let result = parse_mt940_amount("1,00").unwrap();
        assert_eq!(result, Decimal::from_str("1.00").unwrap());
    }
}
