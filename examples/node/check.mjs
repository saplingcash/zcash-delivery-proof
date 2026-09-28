// The WebAssembly package in Node: check every proof in the test vectors with no key, and find each note
// again with the viewing keys. Exits 1 on any difference.
//
//   node examples/node/check.mjs
import { readFileSync } from "node:fs";
import { initSync, check, make, addressHasReceiver } from "../../wasm/pkg/zcash_delivery_proof_wasm.js";

const root = new URL("../../", import.meta.url);
initSync({ module: readFileSync(new URL("wasm/pkg/zcash_delivery_proof_wasm_bg.wasm", root)) });
const read = (f) => JSON.parse(readFileSync(new URL(`test-vectors/${f}`, root), "utf8"));

let failures = 0;
const expect = (ok, what) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${what}`);
  if (!ok) failures++;
};

for (const c of read("constructed.json").cases) {
  for (const p of c.payments) {
    const d = JSON.parse(check(c.txHex, p.proof, c.network));
    expect(d.value === p.value && d.memoText === p.memoText && d.address === p.to && d.pool === c.pool, `${c.name}: ${p.value} zatoshi to ${p.to.slice(0, 16)}…, memo "${d.memoText}"`);
    expect(addressHasReceiver(p.to, p.proof), `${c.name}: the address carries the proof's receiver`);
  }
  const merchant = JSON.parse(make(c.txHex, c.keys.merchantUfvk));
  expect(merchant.length === 1 && merchant[0].side === "received" && merchant[0].proof === c.payments[0].proof, `${c.name}: the merchant's key finds its payment, received`);
  const sender = JSON.parse(make(c.txHex, c.keys.senderUfvk));
  expect(sender.length === 2 && sender.every((f) => f.side === "sent"), `${c.name}: the sender's key finds both payments, sent`);
  let refused = false;
  try {
    check(c.txHex, c.payments[1].proof.slice(0, -2) + "AA", c.network);
  } catch {
    refused = true;
  }
  expect(refused, `${c.name}: a changed proof is refused`);
}

const t = read("testnet.json");
const d = JSON.parse(check(t.txHex, t.proof, t.network));
expect(d.value === 546 && d.txid === t.txid, `testnet ${t.txid.slice(0, 16)}…: 546 zatoshi, memo starts "${(d.memoText ?? "").split("\n")[0]}"`);
expect(JSON.parse(make(t.txHex, t.ufvk))[0]?.proof === t.proof, "testnet: the wallet's key finds its note");

process.exit(failures ? 1 : 0);
