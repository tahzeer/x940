use napi_derive::napi;

use x940rs::{parse_mt940, to_camt053, to_csv, to_json, DecoderChain};

#[napi]
pub struct MT940 {
    statements: Vec<x940rs::Statement>,
    resolver_used: String,
}

#[napi]
impl MT940 {
    #[napi(constructor)]
    pub fn new(text: String, resolver: Option<String>) -> napi::Result<Self> {
        let resolver = resolver.unwrap_or_else(|| "auto".into());
        let chain = DecoderChain::with_resolver(&resolver).unwrap_or_else(DecoderChain::auto);

        let statements = parse_mt940(&text, &chain)
            .map_err(|e| napi::Error::from_reason(format!("Parse error: {}", e)))?;

        Ok(MT940 {
            statements,
            resolver_used: resolver,
        })
    }

    #[napi(getter)]
    pub fn account(&self) -> String {
        self.statements.first().map(|s| s.account_identification.clone()).unwrap_or_default()
    }

    #[napi(getter)]
    pub fn currency(&self) -> String {
        self.statements.first().map(|s| s.opening_balance.currency.clone()).unwrap_or_default()
    }

    #[napi(getter)]
    pub fn opening_balance(&self) -> f64 {
        self.statements
            .first()
            .map(|s| s.opening_balance.amount.to_string().parse().unwrap_or(0.0))
            .unwrap_or(0.0)
    }

    #[napi(getter)]
    pub fn closing_balance(&self) -> f64 {
        self.statements
            .first()
            .map(|s| s.closing_balance.amount.to_string().parse().unwrap_or(0.0))
            .unwrap_or(0.0)
    }

    #[napi(getter)]
    pub fn resolver_used(&self) -> String {
        self.resolver_used.clone()
    }

    #[napi]
    pub fn to_json(&self) -> napi::Result<String> {
        to_json(&self.statements)
            .map_err(|e| napi::Error::from_reason(format!("Export error: {}", e)))
    }

    #[napi]
    pub fn to_csv(&self) -> napi::Result<String> {
        to_csv(&self.statements)
            .map_err(|e| napi::Error::from_reason(format!("Export error: {}", e)))
    }

    #[napi]
    pub fn to_camt053(&self) -> napi::Result<String> {
        to_camt053(&self.statements)
            .map_err(|e| napi::Error::from_reason(format!("Export error: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAYLOAD: &str = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n:61:2401012401D100,00NTRF//REF\r\n:86:test transaction\r\n:62F:C240101EUR900,00\r\n";

    #[test]
    fn parse_basic() {
        let stmt = MT940::new(PAYLOAD.to_string(), Some("auto".into())).unwrap();
        assert_eq!(stmt.account(), "ACCT");
        assert_eq!(stmt.currency(), "EUR");
    }

    #[test]
    fn to_json_output() {
        let stmt = MT940::new(PAYLOAD.to_string(), Some("auto".into())).unwrap();
        let json = stmt.to_json().unwrap();
        assert!(json.contains("transactionReference"));
    }

    #[test]
    fn to_csv_output() {
        let stmt = MT940::new(PAYLOAD.to_string(), Some("auto".into())).unwrap();
        let csv = stmt.to_csv().unwrap();
        assert!(csv.starts_with('\u{FEFF}'));
    }

    #[test]
    fn to_camt053_output() {
        let stmt = MT940::new(PAYLOAD.to_string(), Some("auto".into())).unwrap();
        let xml = stmt.to_camt053().unwrap();
        assert!(xml.contains("camt.053"));
    }

    #[test]
    fn parse_invalid_input() {
        let result = MT940::new(String::new(), Some("auto".into()));
        assert!(result.is_err());
    }

    #[test]
    fn parse_with_resolver() {
        let stmt = MT940::new(PAYLOAD.to_string(), Some("gvc".into())).unwrap();
        assert_eq!(stmt.resolver_used(), "gvc");
    }
}
