use std::collections::HashMap;

mod camt053;
mod csv;
mod json;

pub use self::camt053::to_camt053;
pub use self::csv::to_csv;
pub use self::json::to_json;

pub(crate) fn decimal_to_f64(d: &rust_decimal::Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

pub(crate) fn date_string(d: &chrono::NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

pub(crate) fn date_iso(d: &chrono::NaiveDate) -> String {
    format!("{}T00:00:00.000Z", d.format("%Y-%m-%d"))
}

pub(crate) fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

pub(crate) fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

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

pub(crate) fn resolve_counter_iban(sd: &HashMap<String, String>) -> String {
    sd.get("31").or_else(|| sd.get("30")).or_else(|| sd.get("IBAN")).cloned().unwrap_or_default()
}

pub(crate) fn resolve_purpose(sd: &HashMap<String, String>) -> String {
    let lines: Vec<String> = (20..=29).filter_map(|i| sd.get(&i.to_string())).cloned().collect();
    if !lines.is_empty() {
        return lines.join(" ");
    }
    sd.get("REMI").or_else(|| sd.get("EREF")).cloned().unwrap_or_default()
}
