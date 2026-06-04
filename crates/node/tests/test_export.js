const assert = require("node:assert/strict");
const { MT940 } = require("../index.js");

const SWIFT_PAYLOAD =
    ":20:SWIFTSTRUCT2026\r\n" +
    ":25:EUR8934567890123456\r\n" +
    ":28C:00342/001\r\n" +
    ":60F:C260601EUR50000,00\r\n" +
    ":61:2606010601D1500,00NTRF//INV991\r\n" +
    ":86:/EREF/INV-2026-991/REMI/MONTHLY RETAINER FEES/NAME/ALPHA DIGITAL CORP/BIC/ALPHDEFFXXX\r\n" +
    ":61:2606020602C3250,75NTRF//RFB-882\r\n" +
    ":86:/EREF/TXN-882910/REMI/REIMBURSEMENT FOR OVERHEAD/NAME/BETATECH LOGISTICS/BIC/BETAUS33XXX\r\n" +
    ":62F:C260602EUR51500,75\r\n";

{
    const s = new MT940(SWIFT_PAYLOAD, "auto");
    const data = JSON.parse(s.toJson());
    assert(Array.isArray(data));
    assert.equal(data.length, 1);
    assert.equal(data[0].transactionReference, "SWIFTSTRUCT2026");
    assert.equal(data[0].accountIdentification, "EUR8934567890123456");
    assert.equal(data[0].currency, "EUR");
}

{
    const s = new MT940(SWIFT_PAYLOAD, "auto");
    const data = JSON.parse(s.toJson())[0];
    assert(data.transactions[0].amount < 0);
    assert(data.transactions[1].amount > 0);
    assert.equal(data.transactions[0].structuredDetails.EREF, "INV-2026-991");
    assert.equal(data.transactions[0].structuredDetails.NAME, "ALPHA DIGITAL CORP");
}

{
    const s = new MT940(SWIFT_PAYLOAD, "auto");
    const csv = s.toCsv();
    assert(csv.startsWith("\uFEFF"));
    assert(csv.includes("Statement"));
    assert(csv.includes("-1500.00"));
    const lines = csv.split("\n").filter(l => l.trim().length > 0);
    assert.equal(lines.length, 3);
}

{
    const s = new MT940(SWIFT_PAYLOAD, "auto");
    const xml = s.toCamt053();
    assert(xml.startsWith("<?xml"));
    assert(xml.includes("camt.053"));
    assert(xml.includes("<CdtDbtInd>"));
    assert(xml.includes("<Cdtr>"));
    assert(xml.includes("<Dbtr>"));
}

console.log("All export tests passed");
