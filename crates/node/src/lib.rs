use napi_derive::napi;

use x940rs::{
    amount_to_f64, parse_mt940, to_camt053, to_csv, to_json, DecoderChain, Transaction as CoreTx,
};

#[napi(object)]
#[derive(Clone)]
pub struct Transaction {
    pub date: String,
    pub entry_date: Option<String>,
    pub debit_credit: String,
    pub is_reversal: bool,
    pub amount: f64,
    pub transaction_type: String,
    pub customer_reference: String,
    pub bank_reference: Option<String>,
    pub details: String,
    pub counterparty: Option<String>,
    pub counter_iban: Option<String>,
    pub purpose: Option<String>,
}

impl Transaction {
    fn from_core(tx: &CoreTx) -> Self {
        let signed = tx.signed_amount();
        Transaction {
            date: tx.value_date.format("%Y-%m-%d").to_string(),
            entry_date: tx.entry_date.map(|d| d.format("%Y-%m-%d").to_string()),
            debit_credit: tx.debit_credit.to_string(),
            is_reversal: tx.debit_credit.is_reversal(),
            amount: amount_to_f64(&signed),
            transaction_type: tx.transaction_type.clone(),
            customer_reference: tx.customer_reference.clone(),
            bank_reference: tx.bank_reference.clone(),
            details: tx.details.clone(),
            counterparty: tx.counterparty(),
            counter_iban: tx.counter_iban(),
            purpose: tx.purpose(),
        }
    }
}

#[napi(js_name = "MT940")]
pub struct MT940 {
    #[napi(readonly)]
    pub account: String,
    #[napi(readonly)]
    pub currency: String,
    #[napi(readonly)]
    pub opening_balance: f64,
    #[napi(readonly)]
    pub closing_balance: f64,
    #[napi(readonly)]
    pub resolver_used: String,

    statements: Vec<x940rs::Statement>,
}

#[napi]
impl MT940 {
    #[napi(constructor)]
    pub fn new(text: String, resolver: Option<String>) -> napi::Result<Self> {
        let r = resolver.unwrap_or_else(|| "auto".into());
        let chain = DecoderChain::with_resolver(&r).unwrap_or_else(DecoderChain::auto);

        let statements = parse_mt940(&text, &chain)
            .map_err(|e| napi::Error::from_reason(format!("Parse error: {}", e)))?;

        let first = statements.first();
        Ok(MT940 {
            account: first.map(|s| s.account_identification.clone()).unwrap_or_default(),
            currency: first.map(|s| s.opening_balance.currency.clone()).unwrap_or_default(),
            opening_balance: first.map(|s| amount_to_f64(&s.opening_balance.amount)).unwrap_or(0.0),
            closing_balance: first.map(|s| amount_to_f64(&s.closing_balance.amount)).unwrap_or(0.0),
            resolver_used: r,
            statements,
        })
    }

    #[napi(getter)]
    pub fn transactions(&self) -> Vec<Transaction> {
        self.statements.iter().flat_map(|s| &s.transactions).map(Transaction::from_core).collect()
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
        assert_eq!(stmt.account, "ACCT");
        assert_eq!(stmt.currency, "EUR");
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
        assert_eq!(stmt.resolver_used, "gvc");
    }

    #[test]
    fn transactions_accessible() {
        let stmt = MT940::new(PAYLOAD.to_string(), Some("auto".into())).unwrap();
        let txns = stmt.transactions();
        assert_eq!(txns.len(), 1);
        let tx = &txns[0];
        assert_eq!(tx.debit_credit, "D");
        assert_eq!(tx.amount, -100.0);
        assert_eq!(tx.transaction_type, "NTRF");
        assert_eq!(tx.bank_reference.as_deref(), Some("REF"));
        assert!(!tx.is_reversal);
    }

    #[test]
    fn transactions_with_counterparty() {
        let payload = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n:61:2401012401D100,00NTRF\r\n:86:/NAME/ACME CORP/IBAN/DE1234567890\r\n:62F:C240101EUR900,00\r\n";
        let stmt = MT940::new(payload.to_string(), Some("auto".into())).unwrap();
        let txns = stmt.transactions();
        let tx = &txns[0];
        assert_eq!(tx.counterparty.as_deref(), Some("ACME CORP"));
        assert_eq!(tx.counter_iban.as_deref(), Some("DE1234567890"));
    }
}
