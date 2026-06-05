//! x940 Python binding: high-performance MT940 parser via PyO3

use pyo3::prelude::*;

use x940rs::{amount_to_f64, parse_mt940, to_camt053, to_csv, to_json, DecoderChain};

/// A parsed MT940 transaction exposed to Python.
#[pyclass]
#[derive(Clone)]
struct Transaction {
    value_date: chrono::NaiveDate,
    entry_date: Option<chrono::NaiveDate>,
    debit_credit: String,
    is_reversal: bool,
    amount: rust_decimal::Decimal,
    transaction_type: String,
    customer_reference: String,
    bank_reference: Option<String>,
    details: String,
    structured_details: Option<std::collections::HashMap<String, String>>,
}

#[pymethods]
impl Transaction {
    #[getter]
    fn value_date(&self) -> PyResult<String> {
        Ok(self.value_date.format("%Y-%m-%d").to_string())
    }

    #[getter]
    fn entry_date(&self) -> Option<String> {
        self.entry_date.as_ref().map(|d| d.format("%Y-%m-%d").to_string())
    }

    #[getter]
    fn debit_credit(&self) -> &str {
        &self.debit_credit
    }

    #[getter]
    fn amount(&self) -> f64 {
        let signed = if self.debit_credit == "D" || self.debit_credit == "RC" {
            -self.amount
        } else {
            self.amount
        };
        amount_to_f64(&signed)
    }

    // FIXME: return Decimal, but for now...
    fn signed_amount(&self) -> f64 {
        self.amount()
    }

    #[getter]
    fn is_credit(&self) -> bool {
        self.debit_credit == "C" || self.debit_credit == "RD"
    }

    #[getter]
    fn is_debit(&self) -> bool {
        self.debit_credit == "D" || self.debit_credit == "RC"
    }

    #[getter]
    fn is_reversal(&self) -> bool {
        self.is_reversal
    }

    #[getter]
    fn transaction_type(&self) -> &str {
        &self.transaction_type
    }

    #[getter]
    fn customer_reference(&self) -> &str {
        &self.customer_reference
    }

    #[getter]
    fn bank_reference(&self) -> Option<&str> {
        self.bank_reference.as_deref()
    }

    #[getter]
    fn details(&self) -> &str {
        &self.details
    }

    #[getter]
    fn structured_details(&self) -> Option<std::collections::HashMap<String, String>> {
        self.structured_details.clone()
    }

    #[getter]
    fn counterparty(&self) -> String {
        self.structured_details
            .as_ref()
            .and_then(x940rs::Transaction::resolve_counterparty)
            .unwrap_or_default()
    }

    #[getter]
    fn counter_iban(&self) -> String {
        self.structured_details
            .as_ref()
            .and_then(x940rs::Transaction::resolve_counter_iban)
            .unwrap_or_default()
    }

    #[getter]
    fn purpose(&self) -> String {
        self.structured_details
            .as_ref()
            .and_then(x940rs::Transaction::resolve_purpose)
            .unwrap_or_else(|| self.details.clone())
    }
}

/// A parsed MT940 bank statement.
///
/// Constructed from raw MT940 text. Stores all parsed data in native
/// Rust memory. Supports inspection and export to JSON, CSV, and
/// camt.053 XML without re-parsing.
#[pyclass]
struct MT940 {
    statements: Vec<x940rs::Statement>,
    #[allow(dead_code)]
    raw_text: String,
    resolver_used: String,
}

#[pymethods]
impl MT940 {
    /// Parse a raw MT940 bank statement string.
    ///
    /// Args:
    ///     text: Raw MT940 text as a string
    ///     resolver: (optional) Force a specific Tag 86 dialect decoder.
    ///         Values: "auto" (default), "swift", "gvc", "angular"
    #[new]
    #[pyo3(signature = (text, resolver = None))]
    fn new(text: &str, resolver: Option<&str>) -> PyResult<Self> {
        let resolver = resolver.unwrap_or("auto").to_string();
        let chain = DecoderChain::with_resolver(&resolver).unwrap_or_else(DecoderChain::auto);

        let statements = Python::with_gil(|py| py.allow_threads(|| parse_mt940(text, &chain)))
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        Ok(MT940 {
            statements,
            raw_text: text.to_string(),
            resolver_used: resolver,
        })
    }

    // Inspection

    #[getter]
    fn account(&self) -> &str {
        self.statements.first().map(|s| s.account_identification.as_str()).unwrap_or("")
    }

    #[getter]
    fn currency(&self) -> &str {
        self.statements.first().map(|s| s.opening_balance.currency.as_str()).unwrap_or("")
    }

    #[getter]
    fn opening_balance(&self) -> f64 {
        self.statements.first().map(|s| amount_to_f64(&s.opening_balance.amount)).unwrap_or(0.0)
    }

    #[getter]
    fn closing_balance(&self) -> f64 {
        self.statements.first().map(|s| amount_to_f64(&s.closing_balance.amount)).unwrap_or(0.0)
    }

    #[getter]
    fn resolver_used(&self) -> &str {
        &self.resolver_used
    }

    #[getter]
    fn transactions(&self) -> Vec<Transaction> {
        self.statements
            .first()
            .map(|s| &s.transactions)
            .into_iter()
            .flatten()
            .map(|tx| Transaction {
                value_date: tx.value_date,
                entry_date: tx.entry_date,
                debit_credit: tx.debit_credit.to_string(),
                is_reversal: tx.debit_credit.is_reversal(),
                amount: tx.amount,
                transaction_type: tx.transaction_type.clone(),
                customer_reference: tx.customer_reference.clone(),
                bank_reference: tx.bank_reference.clone(),
                details: tx.details.clone(),
                structured_details: tx.structured_details.clone(),
            })
            .collect()
    }

    // Export

    fn to_json(&self) -> PyResult<String> {
        to_json(&self.statements)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn to_csv(&self) -> PyResult<String> {
        to_csv(&self.statements).map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn to_camt053(&self) -> PyResult<String> {
        to_camt053(&self.statements)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __len__(&self) -> usize {
        self.statements.first().map(|s| s.transactions.len()).unwrap_or(0)
    }
}

#[pymodule]
fn x940(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MT940>()?;
    m.add_class::<Transaction>()?;
    Ok(())
}
