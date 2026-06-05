use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use crate::statement::Statement;

use super::date_string;

type CResult<T> = std::result::Result<T, crate::error::ParseError>;

fn io_err(e: std::io::Error) -> crate::error::ParseError {
    crate::error::ParseError::Parse {
        message: format!("camt.053 XML error: {}", e),
    }
}

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn to_camt053(statements: &[Statement]) -> crate::error::Result<String> {
    let mut buf = Vec::new();
    let mut writer = Writer::new_with_indent(&mut buf, b' ', 2);

    writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None))).map_err(io_err)?;

    let mut doc = BytesStart::new("Document");
    doc.push_attribute(("xmlns", "urn:iso:std:iso:20022:tech:xsd:camt.053.001.06"));
    writer.write_event(Event::Start(doc)).map_err(io_err)?;
    writer.write_event(Event::Start(BytesStart::new("BkToCstmrStmt"))).map_err(io_err)?;

    let creation_time = now_iso();
    let first_ref =
        statements.first().map(|s| s.transaction_reference.as_str()).unwrap_or("export");
    let msg_id = format!("MT940-{}-{}", first_ref, chrono::Utc::now().timestamp_millis());

    // Group header
    writer.write_event(Event::Start(BytesStart::new("GrpHdr"))).map_err(io_err)?;
    write_text_elem(&mut writer, "MsgId", &msg_id)?;
    write_text_elem(&mut writer, "CreDtTm", &creation_time)?;
    writer.write_event(Event::End(BytesEnd::new("GrpHdr"))).map_err(io_err)?;

    for s in statements {
        write_statement(&mut writer, s, &creation_time)?;
    }

    writer.write_event(Event::End(BytesEnd::new("BkToCstmrStmt"))).map_err(io_err)?;
    writer.write_event(Event::End(BytesEnd::new("Document"))).map_err(io_err)?;

    String::from_utf8(buf).map_err(|e| crate::error::ParseError::Parse {
        message: format!("camt.053 encoding error: {}", e),
    })
}

fn write_statement<W: std::io::Write>(
    writer: &mut Writer<W>,
    s: &Statement,
    creation_time: &str,
) -> CResult<()> {
    writer.write_event(Event::Start(BytesStart::new("Stmt"))).map_err(io_err)?;

    write_text_elem(writer, "Id", &s.transaction_reference)?;
    write_text_elem(writer, "CreDtTm", creation_time)?;

    // Account
    writer.write_event(Event::Start(BytesStart::new("Acct"))).map_err(io_err)?;
    writer.write_event(Event::Start(BytesStart::new("Id"))).map_err(io_err)?;
    write_text_elem(writer, "IBAN", &s.account_identification)?;
    writer.write_event(Event::End(BytesEnd::new("Id"))).map_err(io_err)?;
    write_text_elem(writer, "Ccy", &s.opening_balance.currency)?;
    writer.write_event(Event::End(BytesEnd::new("Acct"))).map_err(io_err)?;

    // Balances
    write_balance(writer, "OPBD", &s.opening_balance)?;
    write_balance(writer, "CLBD", &s.closing_balance)?;

    // Transactions
    for (i, tx) in s.transactions.iter().enumerate() {
        write_transaction(writer, s, tx, i)?;
    }

    writer.write_event(Event::End(BytesEnd::new("Stmt"))).map_err(io_err)?;
    Ok(())
}

fn write_balance<W: std::io::Write>(
    writer: &mut Writer<W>,
    code: &str,
    bal: &crate::models::Balance,
) -> CResult<()> {
    let dc = if bal.debit_credit.is_debit() { "DBIT" } else { "CRDT" };
    let amt = format!("{:.2}", bal.amount);

    writer.write_event(Event::Start(BytesStart::new("Bal"))).map_err(io_err)?;

    writer.write_event(Event::Start(BytesStart::new("Tp"))).map_err(io_err)?;
    writer.write_event(Event::Start(BytesStart::new("CdOrPrtry"))).map_err(io_err)?;
    write_text_elem(writer, "Cd", code)?;
    writer.write_event(Event::End(BytesEnd::new("CdOrPrtry"))).map_err(io_err)?;
    writer.write_event(Event::End(BytesEnd::new("Tp"))).map_err(io_err)?;

    let mut amt_tag = BytesStart::new("Amt");
    amt_tag.push_attribute(("Ccy", bal.currency.as_str()));
    write_amt_elem(writer, amt_tag, &amt)?;

    write_text_elem(writer, "CdtDbtInd", dc)?;

    writer.write_event(Event::Start(BytesStart::new("Dt"))).map_err(io_err)?;
    write_text_elem(writer, "Dt", &date_string(&bal.date))?;
    writer.write_event(Event::End(BytesEnd::new("Dt"))).map_err(io_err)?;

    writer.write_event(Event::End(BytesEnd::new("Bal"))).map_err(io_err)?;
    Ok(())
}

fn write_transaction<W: std::io::Write>(
    writer: &mut Writer<W>,
    s: &Statement,
    tx: &crate::models::Transaction,
    idx: usize,
) -> CResult<()> {
    let dc = if tx.debit_credit.is_debit() { "DBIT" } else { "CRDT" };
    let reversal = if tx.debit_credit.is_reversal() { "true" } else { "false" };
    let entry_date = tx.entry_date.as_ref().unwrap_or(&tx.value_date);
    let amt = format!("{:.2}", tx.amount);

    writer.write_event(Event::Start(BytesStart::new("Ntry"))).map_err(io_err)?;

    write_text_elem(writer, "NtryRef", &format!("TXN-{}", idx + 1))?;

    let mut amt_tag = BytesStart::new("Amt");
    amt_tag.push_attribute(("Ccy", s.opening_balance.currency.as_str()));
    write_amt_elem(writer, amt_tag, &amt)?;

    write_text_elem(writer, "CdtDbtInd", dc)?;
    write_text_elem(writer, "RvslInd", reversal)?;

    writer.write_event(Event::Start(BytesStart::new("BookgDt"))).map_err(io_err)?;
    write_text_elem(writer, "Dt", &date_string(entry_date))?;
    writer.write_event(Event::End(BytesEnd::new("BookgDt"))).map_err(io_err)?;

    writer.write_event(Event::Start(BytesStart::new("ValDt"))).map_err(io_err)?;
    write_text_elem(writer, "Dt", &date_string(&tx.value_date))?;
    writer.write_event(Event::End(BytesEnd::new("ValDt"))).map_err(io_err)?;

    writer.write_event(Event::Start(BytesStart::new("BkTxCd"))).map_err(io_err)?;
    writer.write_event(Event::Start(BytesStart::new("Prtry"))).map_err(io_err)?;
    write_text_elem(writer, "Cd", &tx.transaction_type)?;
    writer.write_event(Event::End(BytesEnd::new("Prtry"))).map_err(io_err)?;
    writer.write_event(Event::End(BytesEnd::new("BkTxCd"))).map_err(io_err)?;

    writer.write_event(Event::Start(BytesStart::new("NtryDtls"))).map_err(io_err)?;
    writer.write_event(Event::Start(BytesStart::new("TxDtls"))).map_err(io_err)?;

    let cp_name = tx.counterparty().unwrap_or_default();
    let cp_iban = tx.counter_iban().unwrap_or_default();

    if !cp_name.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("RltdPties"))).map_err(io_err)?;
        if tx.debit_credit.is_debit() {
            writer.write_event(Event::Start(BytesStart::new("Cdtr"))).map_err(io_err)?;
            writer.write_event(Event::Start(BytesStart::new("Pty"))).map_err(io_err)?;
            write_text_elem(writer, "Nm", &cp_name)?;
            writer.write_event(Event::End(BytesEnd::new("Pty"))).map_err(io_err)?;
            writer.write_event(Event::End(BytesEnd::new("Cdtr"))).map_err(io_err)?;
            if !cp_iban.is_empty() {
                writer.write_event(Event::Start(BytesStart::new("CdtrAcct"))).map_err(io_err)?;
                writer.write_event(Event::Start(BytesStart::new("Id"))).map_err(io_err)?;
                write_text_elem(writer, "IBAN", &cp_iban)?;
                writer.write_event(Event::End(BytesEnd::new("Id"))).map_err(io_err)?;
                writer.write_event(Event::End(BytesEnd::new("CdtrAcct"))).map_err(io_err)?;
            }
        } else {
            writer.write_event(Event::Start(BytesStart::new("Dbtr"))).map_err(io_err)?;
            writer.write_event(Event::Start(BytesStart::new("Pty"))).map_err(io_err)?;
            write_text_elem(writer, "Nm", &cp_name)?;
            writer.write_event(Event::End(BytesEnd::new("Pty"))).map_err(io_err)?;
            writer.write_event(Event::End(BytesEnd::new("Dbtr"))).map_err(io_err)?;
            if !cp_iban.is_empty() {
                writer.write_event(Event::Start(BytesStart::new("DbtrAcct"))).map_err(io_err)?;
                writer.write_event(Event::Start(BytesStart::new("Id"))).map_err(io_err)?;
                write_text_elem(writer, "IBAN", &cp_iban)?;
                writer.write_event(Event::End(BytesEnd::new("Id"))).map_err(io_err)?;
                writer.write_event(Event::End(BytesEnd::new("DbtrAcct"))).map_err(io_err)?;
            }
        }
        writer.write_event(Event::End(BytesEnd::new("RltdPties"))).map_err(io_err)?;
    }

    let purpose = tx.purpose().unwrap_or_else(|| tx.details.clone());
    if !purpose.is_empty() {
        writer.write_event(Event::Start(BytesStart::new("RmtInf"))).map_err(io_err)?;
        write_text_elem(writer, "Ustrd", &purpose)?;
        writer.write_event(Event::End(BytesEnd::new("RmtInf"))).map_err(io_err)?;
    }

    writer.write_event(Event::End(BytesEnd::new("TxDtls"))).map_err(io_err)?;
    writer.write_event(Event::End(BytesEnd::new("NtryDtls"))).map_err(io_err)?;
    writer.write_event(Event::End(BytesEnd::new("Ntry"))).map_err(io_err)?;
    Ok(())
}

fn write_text_elem<W: std::io::Write>(
    writer: &mut Writer<W>,
    name: &str,
    text: &str,
) -> CResult<()> {
    writer.create_element(name).write_text_content(BytesText::new(text)).map(|_| ()).map_err(io_err)
}

fn write_amt_elem<W: std::io::Write>(
    writer: &mut Writer<W>,
    start: BytesStart<'_>,
    text: &str,
) -> CResult<()> {
    writer.write_event(Event::Start(start)).map_err(io_err)?;
    writer.write_event(Event::Text(BytesText::new(text))).map_err(io_err)?;
    writer.write_event(Event::End(BytesEnd::new("Amt"))).map_err(io_err)?;
    Ok(())
}
