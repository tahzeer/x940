const assert = require("node:assert/strict");
const { MT940 } = require("../index.js");

const PAYLOAD = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n:61:2401012401D100,00NTRF//REF\r\n:86:test debit transaction\r\n:61:2401012401C50,00NTRF//REF2\r\n:86:test credit transaction\r\n:62F:C240101EUR950,00\r\n";

{
    // parse basic
    const s = new MT940(PAYLOAD, "auto");
    assert.equal(s.account, "ACCT");
    assert.equal(s.currency, "EUR");
    assert.equal(s.openingBalance, 1000.0);
    assert.equal(s.closingBalance, 950.0);
    assert.equal(s.resolverUsed, "auto");
}

{
    // transactions count
    const s = new MT940(PAYLOAD, "auto");
    const txns = s.transactions;
    assert.equal(txns.length, 2);
}

{
    // debit transaction
    const s = new MT940(PAYLOAD, "auto");
    const tx = s.transactions[0];
    assert.equal(tx.transactionType, "NTRF");
    assert.equal(tx.debitCredit, "D");
    assert.equal(tx.isReversal, false);
    assert(tx.amount < 0);
    assert(tx.details.length > 0);
}

{
    // credit transaction
    const s = new MT940(PAYLOAD, "auto");
    const tx = s.transactions[1];
    assert.equal(tx.debitCredit, "C");
    assert(tx.amount > 0);
}

{
    // empty input should error
    assert.throws(() => new MT940("", "auto"), /Parse error/);
}

{
    // resolver
    const s = new MT940(PAYLOAD, "gvc");
    assert.equal(s.resolverUsed, "gvc");
}

{
    // toJSON export
    const s = new MT940(PAYLOAD, "auto");
    const data = JSON.parse(s.toJSON());
    assert(Array.isArray(data));
    assert.equal(data[0].transactionReference, "TEST");
    assert.equal(data[0].transactions.length, 2);
}

{
    // toCSV export
    const s = new MT940(PAYLOAD, "auto");
    const csv = s.toCSV();
    assert(csv.startsWith("\uFEFF"));
    assert(csv.includes("ACCT"));
    assert(csv.includes("-100.00"));
}

{
    // toCamt053 export
    const s = new MT940(PAYLOAD, "auto");
    const xml = s.toCamt053();
    assert(xml.includes("camt.053"));
    assert(xml.includes("<CdtDbtInd>"));
}

{
    // structured details
    const swift = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n:61:2401012401D100,00NTRF\r\n:86:/EREF/INV-001/REMI/TEST/NAME/ACME CORP\r\n:62F:C240101EUR900,00\r\n";
    const s = new MT940(swift, "auto");
    const tx = s.transactions[0];
    assert(tx.structuredDetails !== null);
    assert.equal(tx.counterparty, "ACME CORP");
    assert.equal(tx.purpose, "TEST");
}

console.log("All tests passed");
