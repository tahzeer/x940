import x940 as x


class TestExplicitResolver:
    def test_resolver_swift(self, gvc_payload):
        stmt = x.MT940(gvc_payload, resolver="swift")
        tx = stmt.transactions[0]
        # GVC input with swift resolver: falls through to unstructured
        assert tx.structured_details is not None
        assert "detail" in tx.structured_details

    def test_resolver_gvc(self, swift_payload):
        stmt = x.MT940(swift_payload, resolver="gvc")
        tx = stmt.transactions[0]
        # SWIFT input with gvc resolver: falls through to unstructured
        assert tx.structured_details is not None
        assert "detail" in tx.structured_details

    def test_resolver_angular(self, angular_payload):
        stmt = x.MT940(angular_payload, resolver="angular")
        tx = stmt.transactions[0]
        assert tx.structured_details is not None
        assert tx.structured_details.get("tx_code") == "010"

    def test_resolver_auto_is_same_as_no_resolver(self, swift_payload):
        stmt1 = x.MT940(swift_payload)
        stmt2 = x.MT940(swift_payload, resolver="auto")
        assert stmt1.to_json() == stmt2.to_json()

    def test_explicit_resolver_preserves_unstructured_fallback(self, gvc_payload):
        stmt = x.MT940(gvc_payload, resolver="swift")
        # All transactions should still parse (unstructured fallback)
        assert len(stmt.transactions) == 2
        for tx in stmt.transactions:
            assert tx.structured_details is not None


class TestPerTransactionDetection:
    def test_stress_mixed_dialects(self, stress_payload):
        stmt = x.MT940(stress_payload)
        assert len(stmt.transactions) == 3

        # Tx0: unknown regional format -> unstructured
        t0 = stmt.transactions[0]
        assert "detail" in t0.structured_details

        # Tx1: SWIFT structured -> proper fields
        t1 = stmt.transactions[1]
        assert t1.structured_details["EREF"] == "STRESS-881"
        assert t1.structured_details["NAME"] == "ENTERPRISE HOLDINGS PLC"

        # Tx2: unstructured fallback
        t2 = stmt.transactions[2]
        assert "detail" in t2.structured_details

    def test_stress_multi_line_no_space(self, stress_payload):
        stmt = x.MT940(stress_payload)
        t1 = stmt.transactions[1]
        # "THAT\nSHOULD" -> "THATSHOULD" (no space injected)
        assert "THATSHOULD" in t1.structured_details.get("REMI", "")
        # "THE\nMIDDLE" -> "THEMIDDLE"
        assert "THEMIDDLE" in t1.structured_details.get("REMI", "")

    def test_stress_all_resolvers_work(self, stress_payload):
        for r in ["auto", "swift", "gvc", "angular"]:
            stmt = x.MT940(stress_payload, resolver=r)
            assert len(stmt.transactions) == 3
            # No crash, all txns parsed
            for tx in stmt.transactions:
                assert tx.structured_details is not None
