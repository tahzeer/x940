use std::io::Write;

use crate::error::ParseError;
use crate::statement::Statement;

use super::{csv_escape, date_string};

pub fn to_csv(statements: &[Statement]) -> crate::error::Result<String> {
    let mut buf: Vec<u8> = Vec::new();

    // UTF-8 BOM for Excel compatibility
    buf.extend_from_slice(&[0xEF, 0xBB, 0xBF]);

    writeln!(
        buf,
        "Statement,Account,Currency,Date,EntryDate,Type,Reference,BankRef,Counterparty,CounterIBAN,Purpose,Amount,IsReversal"
    )
    .map_err(|e| ParseError::Parse {
        message: format!("CSV write error: {}", e),
    })?;

    for s in statements {
        let stmt_no = &s.statement_number.statement;
        let acct = &s.account_identification;
        let currency = &s.opening_balance.currency;

        for tx in &s.transactions {
            let signed = tx.signed_amount();
            let amount_str = format!("{:.2}", signed);

            let cp = tx.counterparty().unwrap_or_default();
            let ciban = tx.counter_iban().unwrap_or_default();
            let purpose = tx.purpose().unwrap_or_else(|| tx.details.clone());

            let entry_str = tx
                .entry_date
                .as_ref()
                .map(date_string)
                .unwrap_or_else(|| date_string(&tx.value_date));

            writeln!(
                buf,
                "{},{},{},{},{},{},{},{},{},{},{},{},{}",
                csv_escape(stmt_no),
                csv_escape(acct),
                csv_escape(currency),
                csv_escape(&date_string(&tx.value_date)),
                csv_escape(&entry_str),
                csv_escape(&tx.transaction_type),
                csv_escape(&tx.customer_reference),
                csv_escape(tx.bank_reference.as_deref().unwrap_or("")),
                csv_escape(&cp),
                csv_escape(&ciban),
                csv_escape(&purpose),
                amount_str,
                if tx.debit_credit.is_reversal() { "Y" } else { "N" },
            )
            .map_err(|e| ParseError::Parse {
                message: format!("CSV write error: {}", e),
            })?;
        }
    }

    String::from_utf8(buf).map_err(|e| ParseError::Parse {
        message: format!("CSV encoding error: {}", e),
    })
}
