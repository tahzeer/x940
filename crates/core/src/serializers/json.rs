use serde::Serialize;
use serde_json::Value;

use crate::error::ParseError;
use crate::models::Transaction;
use crate::statement::Statement;

use super::{amount_to_f64, date_iso, date_string};

#[derive(Serialize)]
#[allow(non_snake_case)]
struct JsonTxn {
    date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    entryDate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fundsCode: Option<String>,
    amount: f64,
    isReversal: bool,
    transactionType: String,
    reference: String,
    bankReference: String,
    extraDetails: String,
    currency: String,
    details: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    structuredDetails: Option<Value>,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct JsonStmtNumber {
    statement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    sequence: Option<String>,
    section: String,
}

#[derive(Serialize)]
#[allow(non_snake_case)]
struct JsonStmt {
    transactionReference: String,
    relatedReference: String,
    accountIdentification: String,
    number: JsonStmtNumber,
    statementDate: String,
    openingBalanceDate: String,
    closingBalanceDate: String,
    closingAvailableBalanceDate: String,
    forwardAvailableBalanceDate: String,
    currency: String,
    openingBalance: f64,
    closingBalance: f64,
    closingAvailableBalance: f64,
    forwardAvailableBalance: f64,
    informationToAccountOwner: String,
    transactions: Vec<JsonTxn>,
}

fn txn_to_json(tx: &Transaction, currency: &str) -> JsonTxn {
    let signed = tx.signed_amount();
    let sd_json = tx.structured_details.as_ref().map(|sd| {
        let map: serde_json::map::Map<String, Value> =
            sd.iter().map(|(k, v)| (k.clone(), Value::String(v.clone()))).collect();
        Value::Object(map)
    });

    JsonTxn {
        date: date_string(&tx.value_date),
        entryDate: tx.entry_date.as_ref().map(date_string),
        fundsCode: tx.funds_code.clone(),
        amount: amount_to_f64(&signed),
        isReversal: tx.debit_credit.is_reversal(),
        transactionType: tx.transaction_type.clone(),
        reference: tx.customer_reference.clone(),
        bankReference: tx.bank_reference.clone().unwrap_or_default(),
        extraDetails: tx.supplementary.clone().unwrap_or_default(),
        currency: currency.to_string(),
        details: tx.details.clone(),
        structuredDetails: sd_json,
    }
}

pub fn to_json(statements: &[Statement]) -> crate::error::Result<String> {
    let stmts: Vec<JsonStmt> = statements
        .iter()
        .map(|s| {
            let closing_date = s.closing_balance.date;

            JsonStmt {
                transactionReference: s.transaction_reference.clone(),
                relatedReference: s.related_reference.clone().unwrap_or_default(),
                accountIdentification: s.account_identification.clone(),
                number: JsonStmtNumber {
                    statement: s.statement_number.statement.clone(),
                    sequence: s.statement_number.sequence.clone(),
                    section: String::new(),
                },
                statementDate: date_string(&closing_date),
                openingBalanceDate: date_string(&s.opening_balance.date),
                closingBalanceDate: date_string(&closing_date),
                closingAvailableBalanceDate: s
                    .closing_available
                    .as_ref()
                    .map(|b| date_iso(&b.date))
                    .unwrap_or_else(|| date_iso(&closing_date)),
                forwardAvailableBalanceDate: s
                    .forward_available
                    .as_ref()
                    .map(|b| date_iso(&b.date))
                    .unwrap_or_else(|| date_iso(&closing_date)),
                currency: s.opening_balance.currency.clone(),
                openingBalance: amount_to_f64(&s.opening_balance.amount),
                closingBalance: amount_to_f64(&s.closing_balance.amount),
                closingAvailableBalance: s
                    .closing_available
                    .as_ref()
                    .map(|b| amount_to_f64(&b.amount))
                    .unwrap_or_else(|| amount_to_f64(&s.closing_balance.amount)),
                forwardAvailableBalance: s
                    .forward_available
                    .as_ref()
                    .map(|b| amount_to_f64(&b.amount))
                    .unwrap_or_else(|| amount_to_f64(&s.closing_balance.amount)),
                informationToAccountOwner: s.info_to_owner.clone().unwrap_or_default(),
                transactions: s
                    .transactions
                    .iter()
                    .map(|tx| txn_to_json(tx, &s.opening_balance.currency))
                    .collect(),
            }
        })
        .collect();

    serde_json::to_string_pretty(&stmts).map_err(|e| ParseError::Parse {
        message: format!("JSON serialization error: {}", e),
    })
}
