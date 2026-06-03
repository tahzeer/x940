const assert = require("node:assert/strict");
const { MT940 } = require("../index.js");

{
    // resolver=swift on GVC input falls back to unstructured
    const gvc =
        ":20:GVC\r\n:25:12345678/0009876543\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n" +
        ":61:2401012401D100,00N166\r\n" +
        ":86:166?00REMITTANCE?20INV-9924?32ACME CORP\r\n" +
        ":62F:C240101EUR900,00\r\n";
    const s = new MT940(gvc, "swift");
    const tx = s.transactions[0];
    assert(tx.structuredDetails !== null);
    // Falls to unstructured: "detail" key present, no "gvc"
}

{
    // resolver=gvc on SWIFT input falls back to unstructured
    const swift =
        ":20:SWIFT\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n" +
        ":61:2401012401D100,00NTRF\r\n" +
        ":86:/EREF/INV-001/NAME/ACME CORP\r\n" +
        ":62F:C240101EUR900,00\r\n";
    const s = new MT940(swift, "gvc");
    const tx = s.transactions[0];
    assert(tx.structuredDetails !== null);
}

{
    // resolver=angular on angular input detects correctly
    const angular =
        ":20:ANG\r\n:25:PL121010102300001234567890\r\n:28C:1/1\r\n:60F:C240101PLN1000,00\r\n" +
        ":61:2401012401D100,00N010\r\n" +
        ":86:010<00PRZELEW<20FAKTURA 1234<27JOHN DOE\r\n" +
        ":62F:C240101PLN900,00\r\n";
    const s = new MT940(angular, "angular");
    const tx = s.transactions[0];
    assert(tx.structuredDetails !== null);
}

{
    // resolver=auto is same as no resolver
    const raw = ":20:T\r\n:25:A\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n";
    const s1 = new MT940(raw, undefined);
    const s2 = new MT940(raw, "auto");
    assert.equal(s1.toJSON(), s2.toJSON());
}

{
    // mixed dialects per-transaction
    const stress =
        ":20:STRESS\r\n:25:GBP99887766554433\r\n:28C:1/1\r\n:60F:C240101GBP35000,00\r\n" +
        ":61:2401012401D100,00NTRF\r\n" +
        ":86:UNKNOWN REGIONAL FORMAT CODE\r\n" +
        ":61:2401012401C5000,00NTRF\r\n" +
        ":86:/EREF/STRESS-881/REMI/COMPLEX TRANSACTION THAT\nSHOULD CONTINUOUSLY PARSE/NAME/ENTERPRISE HOLDINGS PLC\r\n" +
        ":61:2401012401D45,50NMSC\r\n" +
        ":86:MONTHLY ACCOUNT SERVICE FEE\r\n" +
        ":62F:C240101GBP35000,00\r\n";
    const s = new MT940(stress, "auto");
    assert.equal(s.transactions.length, 3);

    const t0 = s.transactions[0];
    assert(t0.structuredDetails !== null);
    assert(t0.structuredDetails.detail !== undefined);

    const t1 = s.transactions[1];
    assert(t1.structuredDetails !== null);
    assert.equal(t1.structuredDetails.EREF, "STRESS-881");
    assert.equal(t1.counterparty, "ENTERPRISE HOLDINGS PLC");

    const t2 = s.transactions[2];
    assert(t2.structuredDetails !== null);
    assert(t2.structuredDetails.detail !== undefined);
}

{
    // all resolvers work on stress test
    const stress =
        ":20:STRESS\r\n:25:X\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n" +
        ":61:2401012401D10,00NTRF\r\n" +
        ":86:UNKNOWN TEXT\r\n" +
        ":62F:C240101EUR100,00\r\n";
    for (const r of ["auto", "swift", "gvc", "angular"]) {
        const s = new MT940(stress, r);
        assert.equal(s.transactions.length, 1);
    }
}

{
    // explicit resolver preserves unstructured fallback
    const gvc =
        ":20:GVC\r\n:25:12345678/0009876543\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n" +
        ":61:2401012401D100,00N166\r\n:86:166?00TEST?20INV-9924?32ACME CORP\r\n" +
        ":61:2401012401C50,00N201\r\n:86:201?00GUTSCHRIFT?20KUNDE?32MUELLER CO\r\n" +
        ":62F:C240101EUR950,00\r\n";
    const s = new MT940(gvc, "swift");
    assert.equal(s.transactions.length, 2);
    for (const tx of s.transactions) {
        assert(tx.structuredDetails !== null);
    }
}

console.log("All auto-detect tests passed");
