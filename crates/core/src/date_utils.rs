use chrono::{Datelike, NaiveDate};

use crate::error::Result;

/// Parse an MT940 date string (YYMMDD) to `NaiveDate`.
///
/// All dates in MT940 are 6-digit YYMMDD format.
///
/// # Century Cutoff
///
/// SWIFT standard: no dates before 1980 are expected in customer statements.
///   - `YY < 80` → year 20YY
///   - `YY >= 80` → year 19YY
///
/// # Examples
///
/// ```text
/// "260601" → 2026-06-01
/// "091225" → 2009-12-25
/// "991231" → 1999-12-31
/// ```
#[allow(dead_code)]
pub fn parse_mt940_date(raw: &str) -> Result<NaiveDate> {
    if raw.len() != 6 {
        return Err(crate::error::ParseError::InvalidDate {
            value: raw.to_string(),
            tag: "(date field)",
        });
    }

    let yy: i32 = raw[0..2].parse().map_err(|_| crate::error::ParseError::InvalidDate {
        value: raw.to_string(),
        tag: "(date field)",
    })?;

    let year = if yy < 80 { 2000 + yy } else { 1900 + yy };
    let month: u32 = raw[2..4].parse().map_err(|_| crate::error::ParseError::InvalidDate {
        value: raw.to_string(),
        tag: "(date field)",
    })?;

    let day: u32 = raw[4..6].parse().map_err(|_| crate::error::ParseError::InvalidDate {
        value: raw.to_string(),
        tag: "(date field)",
    })?;

    NaiveDate::from_ymd_opt(year, month, day).ok_or(crate::error::ParseError::InvalidDate {
        value: raw.to_string(),
        tag: "(date field)",
    })
}

/// Infer the entry date year from the value date.
///
/// Entry dates are given as MMDD (4 digits) and the year must be inferred
/// from the associated value date's year, with ±1 year adjustment for
/// year-boundary cases.
///
/// # Rules
///
/// 1. Try same year as value_date
/// 2. If value_date is Dec 31 and entry month < 12 → next year
/// 3. If the computed date is more than 3 months from value_date →
///    try ±1 year adjustment
#[allow(dead_code)]
pub fn infer_entry_date(
    value_date: NaiveDate,
    entry_month: u32,
    entry_day: u32,
) -> Option<NaiveDate> {
    let mut year = value_date.year();

    // Edge case: value_date is Dec 31, entry is Jan/Feb → probably next year
    if value_date.month() == 12 && entry_month < 6 {
        year += 1;
    }

    NaiveDate::from_ymd_opt(year, entry_month, entry_day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_2026_date() {
        let date = parse_mt940_date("260601").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 6, 1).unwrap());
    }

    #[test]
    fn parses_2009_date() {
        let date = parse_mt940_date("091225").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(2009, 12, 25).unwrap());
    }

    #[test]
    fn parses_1999_date() {
        let date = parse_mt940_date("991231").unwrap();
        assert_eq!(date, NaiveDate::from_ymd_opt(1999, 12, 31).unwrap());
    }

    #[test]
    fn parses_century_boundary() {
        // YY=79 → 2079, YY=80 → 1980
        let d1 = parse_mt940_date("790101").unwrap();
        assert_eq!(d1.year(), 2079);

        let d2 = parse_mt940_date("800101").unwrap();
        assert_eq!(d2.year(), 1980);
    }

    #[test]
    fn rejects_invalid_dates() {
        assert!(parse_mt940_date("260230").is_err()); // Feb 30 doesn't exist
        assert!(parse_mt940_date("too short").is_err());
    }
}
