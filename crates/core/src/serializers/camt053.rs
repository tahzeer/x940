use crate::statement::Statement;

use super::{date_string, resolve_counter_iban, resolve_counterparty, resolve_purpose, xml_escape};

pub fn to_camt053(statements: &[Statement]) -> crate::error::Result<String> {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<Document xmlns=\"urn:iso:std:iso:20022:tech:xsd:camt.053.001.06\">\n");
    xml.push_str("  <BkToCstmrStmt>\n");

    xml.push_str("    <GrpHdr>\n");
    xml.push_str("      <MsgId>MT940-EXPORT</MsgId>\n");
    xml.push_str("      <CreDtTm>2026-01-01T00:00:00Z</CreDtTm>\n");
    xml.push_str("    </GrpHdr>\n");

    for s in statements {
        xml.push_str("    <Stmt>\n");
        xml.push_str(&format!("      <Id>{}</Id>\n", xml_escape(&s.transaction_reference)));
        xml.push_str("      <CreDtTm>2026-01-01T00:00:00Z</CreDtTm>\n");

        xml.push_str("      <Acct>\n");
        xml.push_str("        <Id>\n");
        xml.push_str(&format!(
            "          <IBAN>{}</IBAN>\n",
            xml_escape(&s.account_identification)
        ));
        xml.push_str("        </Id>\n");
        xml.push_str(&format!(
            "          <Ccy>{}</Ccy>\n",
            xml_escape(&s.opening_balance.currency)
        ));
        xml.push_str("      </Acct>\n");

        write_balance_xml(&mut xml, "OPBD", &s.opening_balance);
        write_balance_xml(&mut xml, "CLBD", &s.closing_balance);

        for (i, tx) in s.transactions.iter().enumerate() {
            let dc = if tx.debit_credit.is_debit() { "DBIT" } else { "CRDT" };
            let reversal = if tx.debit_credit.is_reversal() { "true" } else { "false" };
            let entry_date = tx.entry_date.as_ref().unwrap_or(&tx.value_date);

            xml.push_str("      <Ntry>\n");
            xml.push_str(&format!("        <NtryRef>TXN-{}</NtryRef>\n", i + 1));
            xml.push_str(&format!(
                "        <Amt Ccy=\"{}\">{:.2}</Amt>\n",
                s.opening_balance.currency, tx.amount
            ));
            xml.push_str(&format!("        <CdtDbtInd>{}</CdtDbtInd>\n", dc));
            xml.push_str(&format!("        <RvslInd>{}</RvslInd>\n", reversal));
            xml.push_str(&format!(
                "        <BookgDt>\n          <Dt>{}</Dt>\n        </BookgDt>\n",
                date_string(entry_date)
            ));
            xml.push_str(&format!(
                "        <ValDt>\n          <Dt>{}</Dt>\n        </ValDt>\n",
                date_string(&tx.value_date)
            ));
            xml.push_str("        <BkTxCd>\n");
            xml.push_str("          <Prtry>\n");
            xml.push_str(&format!("            <Cd>{}</Cd>\n", xml_escape(&tx.transaction_type)));
            xml.push_str("          </Prtry>\n");
            xml.push_str("        </BkTxCd>\n");

            xml.push_str("        <NtryDtls>\n");
            xml.push_str("          <TxDtls>\n");

            let (cp_name, cp_iban) = match &tx.structured_details {
                Some(sd) => (resolve_counterparty(sd), resolve_counter_iban(sd)),
                None => (String::new(), String::new()),
            };

            if !cp_name.is_empty() {
                xml.push_str("            <RltdPties>\n");
                if tx.debit_credit.is_debit() {
                    xml.push_str("              <Cdtr>\n");
                    xml.push_str("                <Pty>\n");
                    xml.push_str(&format!("                  <Nm>{}</Nm>\n", xml_escape(&cp_name)));
                    xml.push_str("                </Pty>\n");
                    xml.push_str("              </Cdtr>\n");
                    if !cp_iban.is_empty() {
                        xml.push_str("              <CdtrAcct>\n");
                        xml.push_str("                <Id>\n");
                        xml.push_str(&format!(
                            "                  <IBAN>{}</IBAN>\n",
                            xml_escape(&cp_iban)
                        ));
                        xml.push_str("                </Id>\n");
                        xml.push_str("              </CdtrAcct>\n");
                    }
                } else {
                    xml.push_str("              <Dbtr>\n");
                    xml.push_str("                <Pty>\n");
                    xml.push_str(&format!("                  <Nm>{}</Nm>\n", xml_escape(&cp_name)));
                    xml.push_str("                </Pty>\n");
                    xml.push_str("              </Dbtr>\n");
                    if !cp_iban.is_empty() {
                        xml.push_str("              <DbtrAcct>\n");
                        xml.push_str("                <Id>\n");
                        xml.push_str(&format!(
                            "                  <IBAN>{}</IBAN>\n",
                            xml_escape(&cp_iban)
                        ));
                        xml.push_str("                </Id>\n");
                        xml.push_str("              </DbtrAcct>\n");
                    }
                }
                xml.push_str("            </RltdPties>\n");
            }

            let purpose = match &tx.structured_details {
                Some(sd) => resolve_purpose(sd),
                None => tx.details.clone(),
            };
            if !purpose.is_empty() {
                xml.push_str("            <RmtInf>\n");
                xml.push_str(&format!("              <Ustrd>{}</Ustrd>\n", xml_escape(&purpose)));
                xml.push_str("            </RmtInf>\n");
            }

            xml.push_str("          </TxDtls>\n");
            xml.push_str("        </NtryDtls>\n");
            xml.push_str("      </Ntry>\n");
        }

        xml.push_str("    </Stmt>\n");
    }

    xml.push_str("  </BkToCstmrStmt>\n");
    xml.push_str("</Document>\n");

    Ok(xml)
}

fn write_balance_xml(buf: &mut String, code: &str, bal: &crate::models::Balance) {
    let dc = if bal.debit_credit.is_debit() { "DBIT" } else { "CRDT" };
    buf.push_str("      <Bal>\n");
    buf.push_str("        <Tp>\n");
    buf.push_str("          <CdOrPrtry>\n");
    buf.push_str(&format!("            <Cd>{}</Cd>\n", code));
    buf.push_str("          </CdOrPrtry>\n");
    buf.push_str("        </Tp>\n");
    buf.push_str(&format!("        <Amt Ccy=\"{}\">{:.2}</Amt>\n", bal.currency, bal.amount));
    buf.push_str(&format!("        <CdtDbtInd>{}</CdtDbtInd>\n", dc));
    buf.push_str(&format!(
        "        <Dt>\n          <Dt>{}</Dt>\n        </Dt>\n",
        date_string(&bal.date)
    ));
    buf.push_str("      </Bal>\n");
}
