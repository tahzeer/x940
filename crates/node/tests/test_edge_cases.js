const assert = require("node:assert/strict");
const { MT940 } = require("../index.js");

{
    // empty input errors
    assert.throws(() => new MT940("", "auto"), /Parse error/);
    assert.throws(() => new MT940("   \n   \n", "auto"), /Parse error/);
}

{
    // single statement, no transactions
    const raw = ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n";
    const s = new MT940(raw, "auto");
    assert.equal(s.transactions.length, 0);
    assert.equal(s.account, "ACCT");
}

{
    // :28C: without sequence number
    const raw = ":20:TEST\r\n:25:ACCT\r\n:28C:00001\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n";
    const s = new MT940(raw, "auto");
    const json = JSON.parse(s.toJSON());
    assert.equal(json[0].number.statement, "00001");
}

{
    // multiple statements
    const raw =
        ":20:STMT1\r\n:25:ACCT1\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n:62F:C240101EUR100,00\r\n" +
        ":20:STMT2\r\n:25:ACCT2\r\n:28C:2/1\r\n:60F:C240102EUR200,00\r\n:62F:C240102EUR200,00\r\n";
    const s = new MT940(raw, "auto");
    assert(s.account.length > 0);
}

{
    // amount with comma decimal
    const raw = ":20:T\r\n:25:A\r\n:28C:1/1\r\n:60F:C240101EUR1000,50\r\n:62F:C240101EUR1000,50\r\n";
    const s = new MT940(raw, "auto");
    assert.equal(s.openingBalance, 1000.50);
}

{
    // amount integer only (no decimal)
    const raw = ":20:T\r\n:25:A\r\n:28C:1/1\r\n:60F:C240101EUR100,\r\n:62F:C240101EUR100,\r\n";
    const s = new MT940(raw, "auto");
    assert.equal(s.openingBalance, 100.0);
}

{
    // reversal debit (RD)
    const raw =
        ":20:TEST\r\n:25:ACCT\r\n:28C:1/1\r\n:60F:C240101EUR1000,00\r\n" +
        ":61:2401012401RD500,00NTRF//INV\r\n" +
        ":86:reversal of debit\r\n" +
        ":62F:C240101EUR1500,00\r\n";
    const s = new MT940(raw, "auto");
    const tx = s.transactions[0];
    assert.equal(tx.debitCredit, "RD");
    assert.equal(tx.isReversal, true);
    assert(tx.amount > 0); // reversal debit = credit = positive
}

{
    // two statements with transactions
    const raw =
        ":20:STMT1\r\n:25:ACC1\r\n:28C:1/1\r\n:60F:C240101EUR100,00\r\n" +
        ":61:2401012401D10,00NTRF\r\n:86:tx1\r\n" +
        ":62F:C240101EUR90,00\r\n" +
        ":20:STMT2\r\n:25:ACC2\r\n:28C:2/1\r\n:60F:C240102EUR200,00\r\n" +
        ":61:2401022402C20,00NTRF\r\n:86:tx2\r\n" +
        ":62F:C240102EUR220,00\r\n";
    const s = new MT940(raw, "auto");
    assert.equal(s.transactions.length, 1); // first statement
}

console.log("All edge case tests passed");
