import x940 as x
import pytest


class TestEdgeCases:
    def test_empty_input(self):
        with pytest.raises(ValueError):
            x.MT940("")

    def test_whitespace_only(self):
        with pytest.raises(ValueError):
            x.MT940("   \n   \n")

    def test_single_statement_no_transactions(self):
        raw = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n"
        stmt = x.MT940(raw)
        assert len(stmt.transactions) == 0
        assert stmt.account == "ACCT"

    def test_number_of_statement_number_only(self):
        # :28C: without sequence number
        raw = ":20:TEST\r\n:25:ACCT\r\n:28C:00001\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n"
        stmt = x.MT940(raw)
        assert stmt.to_json()  # just shouldn't crash

    def test_multiple_statements(self):
        raw = (
            ":20:STMT1\r\n:25:ACCT1\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n"
            ":20:STMT2\r\n:25:ACCT2\r\n:28C:2/1\r\n:60F:C240102EUR200,00\r\n:62F:C240102EUR200,00\r\n"
        )
        stmt = x.MT940(raw)
        assert stmt.account != ""  # first statement accessible

    def test_amount_with_comma_decimal(self):
        raw = ":20:T\r\n:25:A\r\n:28C:1/1\r\n:60F:C240101EUR1000,50\r\n:62F:C240101EUR1000,50\r\n"
        stmt = x.MT940(raw)
        assert stmt.opening_balance == 1000.50

    def test_amount_integer_only(self):
        raw = ":20:T\r\n:25:A\r\n:28C:1/1\r\n:60F:C240101EUR100,\r\n:62F:C240101EUR100,\r\n"
        stmt = x.MT940(raw)
        assert stmt.opening_balance == 100.0

    def test_reversal_debit(self):
        raw = (
            ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n"
            ":61:2401012401RD500,00NTRF//INV\r\n"
            ":86:Reveral of debit\r\n"
            ":62F:C240101EUR1500,00\r\n"
        )
        stmt = x.MT940(raw)
        tx = stmt.transactions[0]
        assert tx.debit_credit == "RD"
        assert tx.is_reversal is True
        # ReversalDebit -> effective credit -> positive amount
        assert tx.amount > 0


class TestMultiStatement:
    def test_two_statements(self):
        raw = (
            ":20:STMT1\r\n:25:ACC1\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n"
            ":61:2401012401D10,00NTRF\r\n:86:tx1\r\n"
            ":62F:C240101EUR90,00\r\n"
            ":20:STMT2\r\n:25:ACC2\r\n:28C:2/1\r\n:60F:C240102EUR200,00\r\n"
            ":61:2401022402C20,00NTRF\r\n:86:tx2\r\n"
            ":62F:C240102EUR220,00\r\n"
        )
        stmt = x.MT940(raw)
        assert len(stmt.transactions) == 1  # first statement's txns
