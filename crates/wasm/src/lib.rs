use wasm_bindgen::prelude::*;

use x940rs::{
    amount_to_f64, parse_mt940, to_camt053, to_csv, to_json, DecoderChain, Transaction as CoreTx,
};

#[wasm_bindgen]
pub struct Transaction {
    date_val: String,
    entry_date_val: Option<String>,
    debit_credit_val: String,
    is_reversal_val: bool,
    amount_val: f64,
    transaction_type_val: String,
    customer_reference_val: String,
    bank_reference_val: Option<String>,
    details_val: String,
    counterparty_val: Option<String>,
    counter_iban_val: Option<String>,
    purpose_val: Option<String>,
}

#[wasm_bindgen]
impl Transaction {
    #[wasm_bindgen(getter, js_name = "date")]
    pub fn date(&self) -> String {
        self.date_val.clone()
    }

    #[wasm_bindgen(getter, js_name = "entryDate")]
    pub fn entry_date(&self) -> Option<String> {
        self.entry_date_val.clone()
    }

    #[wasm_bindgen(getter, js_name = "debitCredit")]
    pub fn debit_credit(&self) -> String {
        self.debit_credit_val.clone()
    }

    #[wasm_bindgen(getter, js_name = "isReversal")]
    pub fn is_reversal(&self) -> bool {
        self.is_reversal_val
    }

    #[wasm_bindgen(getter, js_name = "amount")]
    pub fn amount(&self) -> f64 {
        self.amount_val
    }

    #[wasm_bindgen(getter, js_name = "transactionType")]
    pub fn transaction_type(&self) -> String {
        self.transaction_type_val.clone()
    }

    #[wasm_bindgen(getter, js_name = "customerReference")]
    pub fn customer_reference(&self) -> String {
        self.customer_reference_val.clone()
    }

    #[wasm_bindgen(getter, js_name = "bankReference")]
    pub fn bank_reference(&self) -> Option<String> {
        self.bank_reference_val.clone()
    }

    #[wasm_bindgen(getter, js_name = "details")]
    pub fn details(&self) -> String {
        self.details_val.clone()
    }

    #[wasm_bindgen(getter, js_name = "counterparty")]
    pub fn counterparty(&self) -> Option<String> {
        self.counterparty_val.clone()
    }

    #[wasm_bindgen(getter, js_name = "counterIban")]
    pub fn counter_iban(&self) -> Option<String> {
        self.counter_iban_val.clone()
    }

    #[wasm_bindgen(getter, js_name = "purpose")]
    pub fn purpose(&self) -> Option<String> {
        self.purpose_val.clone()
    }
}

impl Transaction {
    fn from_core(tx: &CoreTx) -> Self {
        let signed = tx.signed_amount();
        Transaction {
            date_val: tx.value_date.format("%Y-%m-%d").to_string(),
            entry_date_val: tx.entry_date.map(|d| d.format("%Y-%m-%d").to_string()),
            debit_credit_val: tx.debit_credit.to_string(),
            is_reversal_val: tx.debit_credit.is_reversal(),
            amount_val: amount_to_f64(&signed),
            transaction_type_val: tx.transaction_type.clone(),
            customer_reference_val: tx.customer_reference.clone(),
            bank_reference_val: tx.bank_reference.clone(),
            details_val: tx.details.clone(),
            counterparty_val: tx.counterparty(),
            counter_iban_val: tx.counter_iban(),
            purpose_val: tx.purpose(),
        }
    }
}

#[wasm_bindgen]
pub struct MT940 {
    account_val: String,
    currency_val: String,
    opening: f64,
    closing: f64,
    resolver_val: String,
    statements: Vec<x940rs::Statement>,
}

#[wasm_bindgen]
impl MT940 {
    #[wasm_bindgen(constructor)]
    pub fn new(text: String, resolver: Option<String>) -> Result<MT940, JsValue> {
        let r = resolver.unwrap_or_else(|| "auto".into());
        let chain = DecoderChain::with_resolver(&r)
            .ok_or_else(|| JsValue::from_str(&format!("unknown resolver: {}", r)))?;

        let statements = parse_mt940(&text, &chain)
            .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

        let first = statements.first();
        Ok(MT940 {
            account_val: first.map(|s| s.account_identification.clone()).unwrap_or_default(),
            currency_val: first.map(|s| s.opening_balance.currency.clone()).unwrap_or_default(),
            opening: first.map(|s| amount_to_f64(&s.opening_balance.amount)).unwrap_or(0.0),
            closing: first.map(|s| amount_to_f64(&s.closing_balance.amount)).unwrap_or(0.0),
            resolver_val: r,
            statements,
        })
    }

    #[wasm_bindgen(getter, js_name = "account")]
    pub fn account(&self) -> String {
        self.account_val.clone()
    }

    #[wasm_bindgen(getter, js_name = "currency")]
    pub fn currency(&self) -> String {
        self.currency_val.clone()
    }

    #[wasm_bindgen(getter, js_name = "openingBalance")]
    pub fn opening_balance(&self) -> f64 {
        self.opening
    }

    #[wasm_bindgen(getter, js_name = "closingBalance")]
    pub fn closing_balance(&self) -> f64 {
        self.closing
    }

    #[wasm_bindgen(getter, js_name = "resolverUsed")]
    pub fn resolver_used(&self) -> String {
        self.resolver_val.clone()
    }

    pub fn transactions(&self) -> Vec<Transaction> {
        self.statements.iter().flat_map(|s| &s.transactions).map(Transaction::from_core).collect()
    }

    pub fn to_json(&self) -> Result<String, JsValue> {
        to_json(&self.statements).map_err(|e| JsValue::from_str(&format!("Export error: {}", e)))
    }

    pub fn to_csv(&self) -> Result<String, JsValue> {
        to_csv(&self.statements).map_err(|e| JsValue::from_str(&format!("Export error: {}", e)))
    }

    pub fn to_camt053(&self) -> Result<String, JsValue> {
        to_camt053(&self.statements).map_err(|e| JsValue::from_str(&format!("Export error: {}", e)))
    }
}
