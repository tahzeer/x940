import x940 as x


class TestMT940Construction:
    def test_parses_account_identification(self, swift_payload):
        stmt = x.MT940(swift_payload)
        assert stmt.account == "EUR8934567890123456"

    def test_parses_currency(self, swift_payload):
        stmt = x.MT940(swift_payload)
        assert stmt.currency == "EUR"

    def test_parses_opening_balance(self, swift_payload):
        stmt = x.MT940(swift_payload)
        assert stmt.opening_balance == 50000.00

    def test_parses_closing_balance(self, swift_payload):
        stmt = x.MT940(swift_payload)
        assert stmt.closing_balance == 51500.75

    def test_parses_transaction_count(self, swift_payload):
        stmt = x.MT940(swift_payload)
        assert len(stmt.transactions) == 3

    def test_resolver_default_is_auto(self, swift_payload):
        stmt = x.MT940(swift_payload)
        assert stmt.resolver_used == "auto"

    def test_explicit_resolver_gvc(self, gvc_payload):
        stmt = x.MT940(gvc_payload, resolver="gvc")
        assert stmt.resolver_used == "gvc"


class TestTransactionProperties:
    def test_debit_amount_is_negative(self, swift_payload):
        stmt = x.MT940(swift_payload)
        tx = stmt.transactions[0]
        assert tx.debit_credit == "D"
        assert tx.amount < 0

    def test_credit_amount_is_positive(self, swift_payload):
        stmt = x.MT940(swift_payload)
        tx = stmt.transactions[1]
        assert tx.debit_credit == "C"
        assert tx.amount > 0

    def test_transaction_type_preserved(self, swift_payload):
        stmt = x.MT940(swift_payload)
        assert stmt.transactions[0].transaction_type == "NTRF"
        assert stmt.transactions[2].transaction_type == "NMSC"

    def test_value_date_parsed(self, swift_payload):
        stmt = x.MT940(swift_payload)
        assert stmt.transactions[0].value_date == "2026-06-01"

    def test_debit_is_credit_false(self, swift_payload):
        stmt = x.MT940(swift_payload)
        tx = stmt.transactions[0]
        assert tx.is_credit is False
        assert tx.is_debit is True

    def test_credit_is_credit_true(self, swift_payload):
        stmt = x.MT940(swift_payload)
        tx = stmt.transactions[1]
        assert tx.is_credit is True
        assert tx.is_debit is False

    def test_structured_details_populated_swift(self, swift_payload):
        stmt = x.MT940(swift_payload)
        tx = stmt.transactions[0]
        assert tx.structured_details is not None
        assert tx.structured_details["EREF"] == "INV-2026-991"
        assert tx.structured_details["NAME"] == "ALPHA DIGITAL CORP"

    def test_counterparty_resolved_swift(self, swift_payload):
        stmt = x.MT940(swift_payload)
        assert stmt.transactions[0].counterparty == "ALPHA DIGITAL CORP"

    def test_counterparty_resolved_gvc(self, gvc_payload):
        stmt = x.MT940(gvc_payload)
        assert stmt.transactions[0].counterparty == "ACME CORP GMBH"

    def test_purpose_resolved_swift(self, swift_payload):
        stmt = x.MT940(swift_payload)
        assert stmt.transactions[0].purpose == "MONTHLY RETAINER FEES"

    def test_purpose_resolved_gvc(self, gvc_payload):
        stmt = x.MT940(gvc_payload)
        purpose = stmt.transactions[0].purpose
        assert "INV-9924" in purpose
        assert "KREATOR ABSCHNITT 1" in purpose


class TestGvcDialect:
    def test_gvc_code_extracted(self, gvc_payload):
        stmt = x.MT940(gvc_payload)
        tx = stmt.transactions[0]
        assert tx.structured_details["gvc"] == "166"

    def test_gvc_subfields_extracted(self, gvc_payload):
        stmt = x.MT940(gvc_payload)
        tx = stmt.transactions[0]
        assert tx.structured_details["30"] == "10020030"
        assert tx.structured_details["31"] == "88776655"

    def test_gvc_second_tx(self, gvc_payload):
        stmt = x.MT940(gvc_payload)
        tx = stmt.transactions[1]
        assert tx.structured_details["gvc"] == "201"
        assert tx.structured_details["32"] == "MUELLER TRADING CO"


class TestAngularDialect:
    def test_angular_tx_code_extracted(self, angular_payload):
        stmt = x.MT940(angular_payload)
        tx = stmt.transactions[0]
        assert tx.structured_details["tx_code"] == "010"

    def test_angular_subfields_extracted(self, angular_payload):
        stmt = x.MT940(angular_payload)
        tx = stmt.transactions[0]
        assert tx.structured_details["20"] == "FAKTURA 1234/2026"
        assert tx.structured_details["27"] == "JOHN DOE SERVICES"

    def test_angular_currency_pln(self, angular_payload):
        stmt = x.MT940(angular_payload)
        assert stmt.currency == "PLN"


class TestUnstructuredDialect:
    def test_unstructured_detail_key(self, unstructured_payload):
        stmt = x.MT940(unstructured_payload)
        tx = stmt.transactions[0]
        assert tx.structured_details is not None
        assert "detail" in tx.structured_details
        assert "WIRE TRANSFER" in tx.structured_details["detail"]

    def test_unstructured_counterparty_empty(self, unstructured_payload):
        stmt = x.MT940(unstructured_payload)
        assert stmt.transactions[0].counterparty == ""
