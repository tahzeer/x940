import json
import x940 as x


class TestJsonExport:
    def test_to_json_is_valid(self, swift_payload):
        stmt = x.MT940(swift_payload)
        data = json.loads(stmt.to_json())
        assert isinstance(data, list)
        assert len(data) == 1

    def test_json_statement_fields(self, swift_payload):
        stmt = x.MT940(swift_payload)
        data = json.loads(stmt.to_json())[0]
        assert data["transactionReference"] == "SWIFTSTRUCT2026"
        assert data["accountIdentification"] == "EUR8934567890123456"
        assert data["currency"] == "EUR"

    def test_json_signed_amounts(self, swift_payload):
        stmt = x.MT940(swift_payload)
        data = json.loads(stmt.to_json())[0]
        txns = data["transactions"]
        assert txns[0]["amount"] < 0  # debit
        assert txns[1]["amount"] > 0  # credit

    def test_json_structured_details(self, swift_payload):
        stmt = x.MT940(swift_payload)
        data = json.loads(stmt.to_json())[0]
        sd = data["transactions"][0]["structuredDetails"]
        assert sd["EREF"] == "INV-2026-991"
        assert sd["NAME"] == "ALPHA DIGITAL CORP"

    def test_json_gvc_structured_details(self, gvc_payload):
        stmt = x.MT940(gvc_payload)
        data = json.loads(stmt.to_json())[0]
        sd = data["transactions"][0]["structuredDetails"]
        assert sd["gvc"] == "166"

    def test_json_angular_structured_details(self, angular_payload):
        stmt = x.MT940(angular_payload)
        data = json.loads(stmt.to_json())[0]
        sd = data["transactions"][0]["structuredDetails"]
        assert sd["tx_code"] == "010"

    def test_json_unstructured_has_detail(self, unstructured_payload):
        stmt = x.MT940(unstructured_payload)
        data = json.loads(stmt.to_json())[0]
        sd = data["transactions"][0]["structuredDetails"]
        assert "detail" in sd


class TestCsvExport:
    def test_to_csv_has_bom(self, swift_payload):
        stmt = x.MT940(swift_payload)
        csv = stmt.to_csv()
        assert csv.startswith("\ufeff")

    def test_to_csv_has_header(self, swift_payload):
        stmt = x.MT940(swift_payload)
        csv = stmt.to_csv()
        lines = csv.split("\n")
        assert "Statement" in lines[0]
        assert "Counterparty" in lines[0]
        assert "Amount" in lines[0]

    def test_to_csv_three_rows(self, swift_payload):
        stmt = x.MT940(swift_payload)
        csv = stmt.to_csv()
        lines = [l for l in csv.split("\n") if l.strip()]
        assert len(lines) == 4  # header + 3 txns

    def test_to_csv_signed_amounts(self, swift_payload):
        stmt = x.MT940(swift_payload)
        csv = stmt.to_csv()
        rows = [l for l in csv.split("\n") if l.strip()]
        assert "-1500.00" in rows[1]  # debit
        assert "3250.75" in rows[2]   # credit


class TestCamt053Export:
    def test_to_camt053_is_xml(self, swift_payload):
        stmt = x.MT940(swift_payload)
        xml = stmt.to_camt053()
        assert xml.startswith('<?xml')
        assert 'camt.053' in xml

    def test_to_camt053_has_db_it(self, swift_payload):
        stmt = x.MT940(swift_payload)
        xml = stmt.to_camt053()
        assert "<CdtDbtInd>" in xml

    def test_to_camt053_has_ntry_dtls(self, swift_payload):
        stmt = x.MT940(swift_payload)
        xml = stmt.to_camt053()
        assert "<NtryDtls>" in xml

    def test_to_camt053_debit_routes_to_cdtr(self, swift_payload):
        stmt = x.MT940(swift_payload)
        xml = stmt.to_camt053()
        # debit entries should have Cdtr (Creditor) blocks
        assert "<Cdtr>" in xml

    def test_to_camt053_credit_routes_to_dbtr(self, swift_payload):
        stmt = x.MT940(swift_payload)
        xml = stmt.to_camt053()
        # credit entries should have Dbtr (Debtor) blocks
        assert "<Dbtr>" in xml
