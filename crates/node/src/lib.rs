use napi_derive::napi;

use x940rs::{parse_mt940, to_camt053, to_csv, to_json, DecoderChain};

fn resolve_counterparty(sd: &std::collections::HashMap<String, String>) -> String {
    if let Some(n) = sd.get("32") {
        if let Some(c) = sd.get("33") {
            return format!("{} {}", n, c);
        }
        return n.clone();
    }
    sd.get("27").or_else(|| sd.get("NAME")).cloned().unwrap_or_default()
}

fn resolve_counter_iban(sd: &std::collections::HashMap<String, String>) -> String {
    sd.get("31").or_else(|| sd.get("30")).or_else(|| sd.get("IBAN")).cloned().unwrap_or_default()
}

fn resolve_purpose(tx: &x940rs::Transaction) -> String {
    match &tx.structured_details {
        Some(sd) => {
            let lines: Vec<String> =
                (20..=29).filter_map(|i| sd.get(&i.to_string())).cloned().collect();
            if !lines.is_empty() {
                return lines.join(" ");
            }
            sd.get("REMI").or_else(|| sd.get("EREF")).cloned().unwrap_or_default()
        }
        None => tx.details.clone(),
    }
}

fn sd_to_json(sd: &std::collections::HashMap<String, String>) -> serde_json::Value {
    let map: serde_json::map::Map<String, serde_json::Value> =
        sd.iter().map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone()))).collect();
    serde_json::Value::Object(map)
}

#[napi(object)]
pub struct Transaction {
    pub value_date: String,
    pub entry_date: Option<String>,
    pub debit_credit: String,
    pub is_reversal: bool,
    pub amount: f64,
    pub transaction_type: String,
    pub customer_reference: String,
    pub bank_reference: Option<String>,
    pub details: String,
    pub structured_details: serde_json::Value,
    pub counterparty: String,
    pub counter_iban: String,
    pub purpose: String,
}

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

    #[napi(getter)]
    pub fn transactions(&self) -> Vec<Transaction> {
        self.statements
            .iter()
            .flat_map(|s| &s.transactions)
            .map(|tx| {
                let signed = tx.signed_amount();
                Transaction {
                    value_date: tx.value_date.format("%Y-%m-%d").to_string(),
                    entry_date: tx.entry_date.as_ref().map(|d| d.format("%Y-%m-%d").to_string()),
                    debit_credit: tx.debit_credit.to_string(),
                    is_reversal: tx.debit_credit.is_reversal(),
                    amount: signed.to_string().parse().unwrap_or(0.0),
                    transaction_type: tx.transaction_type.clone(),
                    customer_reference: tx.customer_reference.clone(),
                    bank_reference: tx.bank_reference.clone(),
                    details: tx.details.clone(),
                    structured_details: tx
                        .structured_details
                        .as_ref()
                        .map(sd_to_json)
                        .unwrap_or(serde_json::Value::Null),
                    counterparty: tx
                        .structured_details
                        .as_ref()
                        .map(resolve_counterparty)
                        .unwrap_or_default(),
                    counter_iban: tx
                        .structured_details
                        .as_ref()
                        .map(resolve_counter_iban)
                        .unwrap_or_default(),
                    purpose: resolve_purpose(tx),
                }
            })
            .collect()
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
        assert_eq!(stmt.opening_balance(), 1000.00);
        assert_eq!(stmt.closing_balance(), 900.00);
    }

    #[test]
    fn parse_transactions_count() {
        let stmt = MT940::new(PAYLOAD.to_string(), Some("auto".into())).unwrap();
        assert_eq!(stmt.transactions().len(), 1);
    }

    #[test]
    fn parse_transaction_properties() {
        let stmt = MT940::new(PAYLOAD.to_string(), Some("auto".into())).unwrap();
        let tx = &stmt.transactions()[0];
        assert_eq!(tx.transaction_type, "NTRF");
        assert_eq!(tx.debit_credit, "D");
        assert!(!tx.is_reversal);
        assert!(tx.amount < 0.0);
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

    #[test]
    fn to_json_output() {
        let stmt = MT940::new(PAYLOAD.to_string(), Some("auto".into())).unwrap();
        let json = stmt.to_json().unwrap();
        assert!(json.contains("transactionReference"));
        assert!(json.contains("ACCT"));
    }

    #[test]
    fn to_csv_output() {
        let stmt = MT940::new(PAYLOAD.to_string(), Some("auto".into())).unwrap();
        let csv = stmt.to_csv().unwrap();
        assert!(csv.starts_with('\u{FEFF}'));
        assert!(csv.contains("ACCT"));
    }

    #[test]
    fn to_camt053_output() {
        let stmt = MT940::new(PAYLOAD.to_string(), Some("auto".into())).unwrap();
        let xml = stmt.to_camt053().unwrap();
        assert!(xml.contains("camt.053"));
        assert!(xml.contains("<CdtDbtInd>"));
    }
}
