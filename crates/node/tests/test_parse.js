const assert = require("node:assert/strict");
const { MT940 } = require("../index.js");

const PAYLOAD = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n:61:2401012401D100,00NTRF//REF\r\n:86:test debit transaction\r\n:61:2401012401C50,00NTRF//REF2\r\n:86:test credit transaction\r\n:62F:C240101EUR950,00\r\n";

{
    const s = new MT940(PAYLOAD, "auto");
    assert.equal(s.account, "ACCT");
    assert.equal(s.currency, "EUR");
    assert.equal(s.openingBalance, 1000.0);
    assert.equal(s.closingBalance, 950.0);
    assert.equal(s.resolverUsed, "auto");
}

{
    const s = new MT940(PAYLOAD, "auto");
    const data = JSON.parse(s.toJson());
    assert(Array.isArray(data));
    assert.equal(data[0].transactions.length, 2);
}

{
    const s = new MT940(PAYLOAD, "auto");
    const data = JSON.parse(s.toJson())[0];
    assert.equal(data.transactionReference, "TEST");
    assert(data.transactions[0].amount < 0);
    assert(data.transactions[0].isReversal === false);
    assert(data.transactions[1].amount > 0);
}

{
    assert.throws(() => new MT940("", "auto"), /Parse error/);
}

{
    const s = new MT940(PAYLOAD, "gvc");
    assert.equal(s.resolverUsed, "gvc");
}

{
    const s = new MT940(PAYLOAD, "auto");
    const csv = s.toCsv();
    assert(csv.startsWith("\uFEFF"));
    assert(csv.includes("ACCT"));
    assert(csv.includes("-100.00"));
}

{
    const s = new MT940(PAYLOAD, "auto");
    const xml = s.toCamt053();
    assert(xml.includes("camt.053"));
    assert(xml.includes("<CdtDbtInd>"));
}

{
    const swift = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n:61:2401012401D100,00NTRF\r\n:86:/EREF/INV-001/REMI/TEST/NAME/ACME CORP\r\n:62F:C240101EUR900,00\r\n";
    const s = new MT940(swift, "auto");
    const data = JSON.parse(s.toJson())[0];
    assert.equal(data.transactions[0].structuredDetails.NAME, "ACME CORP");
    assert.equal(data.transactions[0].structuredDetails.EREF, "INV-001");
}

console.log("All tests passed");
