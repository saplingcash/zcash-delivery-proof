# zcash-delivery-proof: specification, version 1

A **delivery proof** shows that one Zcash transaction delivers one shielded note, in the Orchard, the
Ironwood or the Sapling pool, to one receiver, with a value and a memo. It shows nothing else about the
wallet that received or sent it. Anyone can check it against the transaction's bytes with no key.

The key words MUST, MUST NOT and MAY are used as in RFC 2119.

## 1. What a proof states

"The transaction with this txid carries, in this pool's bundle at this action index, a note of this value
to this receiver, and the note's memo is this one." In the Sapling pool the action is an output
description, and its index is the output's index in the Sapling bundle.

A check establishes the statement for the transaction bytes it is given. It does not establish:

- that those bytes are mined: look the txid up on a node or explorer, or compare the wtxid, which also
  covers signatures and proofs (§4);
- who sent the note: the proof names the receiver only;
- that the bundle's zk-SNARK and signatures are valid: every mined transaction's are, and the optional
  bundle check (§6) verifies an Orchard or Ironwood bundle's offline.

## 2. Format

A proof is 118 bytes:

| Offset | Size | Field |
|---|---|---|
| 0 | 32 | txid, in internal byte order (reversed from how explorers display it) |
| 32 | 1 | pool: `1` = Orchard, `2` = Ironwood, `3` = Sapling |
| 33 | 2 | action index in the pool's bundle (Sapling: output index), u16 little-endian |
| 35 | 43 | receiver: the raw address in the pool's protocol, an 11-byte diversifier then a 32-byte pk_d (an Orchard-family address for pools 1 and 2, a Sapling payment address for pool 3) |
| 78 | 8 | value in zatoshi, u64 little-endian |
| 86 | 32 | rseed |

As text it is `zdp:1:` (this format and its version) followed by the 118 bytes in base64url without
padding. A proof whose pool byte is not 1, 2 or 3, or whose length differs, MUST be refused.

The receiver is shown to people as a unified address holding only that receiver (ZIP 316): an Orchard
receiver for pools 1 and 2, a Sapling receiver for pool 3. The address a payer was given may carry more
receivers; it matches when one of them is this receiver in the proof's pool. A Sapling receiver also
matches a Sapling address (`zs…`) that is exactly it.

Pool 3 was added after the first release of this version. A checker that predates it refuses such a proof
as malformed (it knows pools 1 and 2 only), so no checker reads a Sapling proof as an Orchard-family one.

## 3. Making a proof

A wallet MAY make a proof for any note it can see:

- **Received**, with an incoming viewing key (from a UFVK or UIVK, external or internal scope): trial
  decryption of each action's `enc_ciphertext` gives the note (receiver, value, rseed) and its memo.
- **Sent**, with an outgoing viewing key (from a UFVK, or the 32 bytes of an outgoing viewing key
  alone): recovery from each action's `out_ciphertext` gives the same. This works for notes the sending
  wallet built with its outgoing viewing key, as wallets do by default. A sender MAY publish an outgoing
  viewing key so that anyone can find and prove what it sent with it; that key sees nothing received.

Either way the proof is the same: txid, pool, action index, and the note's receiver, value and rseed.
A UFVK's or UIVK's Orchard item sees the Orchard and the Ironwood pools, its Sapling item the Sapling
pool. An Orchard note has plaintext version 2 (lead byte `0x02`, ZIP 212), an Ironwood note version 3
(lead byte `0x03`), a Sapling note version 2 (lead byte `0x02`, ZIP 212). A Sapling note with plaintext
version 1 (made before ZIP 212) has no rseed, only a random `rcm` and `esk`, and cannot be proved.

## 4. Checking a proof

Given the transaction's bytes and a proof, a checker MUST do all of the following, and the proof holds
only if every step passes:

1. **The transaction.** Parse the bytes as a Zcash transaction under the consensus branch its header
   names (a v4 transaction, which names none, under Canopy). Serialise it again: the result MUST equal the
   bytes given. Its txid MUST equal the proof's.
2. **The action.** The transaction MUST have a bundle in the proof's pool, and that bundle an action at
   the proof's index (Sapling: an output description at that index).
3. **The note.**
   - Orchard and Ironwood: let `rho` be the action's nullifier (as the note's rho) and build the note
     from the proof's receiver, value and rseed, with the pool's note plaintext version. Its extracted
     note commitment MUST equal the action's `cmx`.
   - Sapling: the receiver MUST be a valid Sapling payment address (its diversifier has a base point
     `g_d`, its `pk_d` is a valid point). Build the note from the receiver, the value and the rseed (ZIP
     212: `rcm` is derived from the rseed). Its note commitment's u-coordinate MUST equal the output's
     `cmu`.
4. **The encryption.** Derive the ephemeral secret key `esk` from the note (ZIP 212), and decrypt the
   action's `enc_ciphertext` with `esk` and the receiver's `pk_d` (output recovery with `pk_d` and `esk`,
   in the pool's note encryption domain). It MUST decrypt, and to exactly the proof's receiver, value and
   rseed. The note encryption also checks that `esk` gives the action's `epk`, and that the decrypted
   note has the action's note commitment.

The decrypted memo is the note's memo: the ciphertext is authenticated (ChaCha20-Poly1305), so a memo
that decrypts under the note's key is the one the sender encrypted. Changing any byte of the ciphertext
changes the txid (step 1) and, under a proof naming the new txid, fails step 4.

A Sapling note commitment does not depend on the note's position (there is no rho): the proof is pinned
to its output by the txid and the output index, and step 4 ties the note to that output's ciphertext and
ephemeral key.

The result is the txid, the wtxid (ZIP 239: the txid then the authorizing-data digest), the pool, the
action, the receiver, the value and the 512-byte memo. A memo whose first byte is at most `0xF4` is text
(ZIP 302): UTF-8, padded with zero bytes.

## 5. What a proof reveals

- The receiver: one diversified address. Anyone holding the proof can link other payments to that same
  address (a wallet can give each payer its own diversified address).
- The value and the memo of that one note, and which transaction and action carried it.

It reveals no viewing key, no spending key, and no other note: each note needs its own rseed. It does
not reveal whether or when the note was spent: the nullifier that spends a note needs the wallet's
nullifier key, which the proof does not contain.

## 6. The bundle check (optional)

A checker MAY also verify an Orchard or Ironwood bundle offline as a node does: the zk-SNARK, every
action's spend-authorisation signature and the binding signature, against the transaction's shielded
signature hash (ZIP 244), with the verifying key for the bundle's circuit version. For a transaction with
transparent inputs that hash commits to the values and scripts of the coins those inputs spend, which the
transaction does not carry: the checker MUST be given them. A wrong value or script can only make a valid
bundle fail.

A Sapling bundle's proofs are checked against the Sapling circuits' parameters; this check does not cover
them, and MUST refuse a Sapling bundle rather than pass it.

## 7. Versioning

`zdp:1:` is this version. A later format would use another version number; a checker MUST refuse a
version it does not know.
