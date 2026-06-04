const assert = require("node:assert/strict");
const { MT940 } = require("../index.js");

{
    assert.throws(() => new MT940("", "auto"), /Parse error/);
}

{
    const raw = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n";
    const s = new MT940(raw, "auto");
    const data = JSON.parse(s.toJson())[0];
    assert.equal(data.transactions.length, 0);
    assert.equal(s.account, "ACCT");
}

{
    const raw = ":20:TEST\r\n:25:ACCT\r\n:28C:00001\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n";
    const s = new MT940(raw, "auto");
    const data = JSON.parse(s.toJson())[0];
    assert.equal(data.number.statement, "00001");
}

{
    const raw =
        ":20:STMT1\r\n:25:ACCT1\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n" +
        ":20:STMT2\r\n:25:ACCT2\r\n:28C:2/1\r\n:60F:C240102EUR200,00\r\n:62F:C240102EUR200,00\r\n";
    const s = new MT940(raw, "auto");
    assert(s.account.length > 0);
}

{
    const raw = ":20:T\r\n:25:A\r\n:28C:1/1\r\n:60F:C240101EUR1000,50\r\n:62F:C240101EUR1000,50\r\n";
    const s = new MT940(raw, "auto");
    assert.equal(s.openingBalance, 1000.50);
}

{
    const raw =
        ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n" +
        ":61:2401012401RD500,00NTRF//INV\r\n" +
        ":86:reversal of debit\r\n" +
        ":62F:C240101EUR1500,00\r\n";
    const s = new MT940(raw, "auto");
    const data = JSON.parse(s.toJson())[0];
    const tx = data.transactions[0];
    assert.equal(tx.isReversal, true);
    assert(tx.amount > 0);
}

console.log("All edge case tests passed");
