mod camt053;
mod csv;
mod json;

pub use self::camt053::to_camt053;
pub use self::csv::to_csv;
pub use self::json::to_json;

pub fn amount_to_f64(d: &rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

pub(crate) fn date_string(d: &chrono::NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub(crate) fn date_iso(d: &chrono::NaiveDate) -> String {
    format!("{}T00:00:00.000Z", d.format("%Y-%m-%d"))
}

pub(crate) fn csv_escape(s: &str) -> String {
    let needs_prefix =
        s.starts_with('=') || s.starts_with('+') || s.starts_with('-') || s.starts_with('@');
    let escaped = if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    };
    if needs_prefix {
        format!("\t{}", escaped)
    } else {
        escaped
    }
}
