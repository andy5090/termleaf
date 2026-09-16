import assert from "node:assert/strict";
import test from "node:test";

import {
  decodeEnvelope,
  encodeEnvelope,
  inspectJournal,
} from "./storageEnvelope.ts";

const validate = (payload: string) => {
  const parsed: unknown = JSON.parse(payload);
  if (typeof parsed !== "object" || parsed === null) throw new Error("invalid payload");
};

test("envelopes retain Unicode payloads and reject partial writes", () => {
  const payload = JSON.stringify({ body: "한글\n日本語 👩‍💻" });
  const encoded = encodeEnvelope(4, payload);

  assert.equal(decodeEnvelope(encoded, validate).payload, payload);
  assert.throws(() => decodeEnvelope(encoded.slice(0, -5), validate));
});

test("an interrupted newer slot falls back to the prior valid snapshot", () => {
  const prior = encodeEnvelope(7, JSON.stringify({ body: "still safe" }));
  const interrupted = encodeEnvelope(8, JSON.stringify({ body: "new" })).slice(0, -9);

  const journal = inspectJournal([prior, interrupted], validate);
  assert.equal(journal.latest?.sequence, 7);
  assert.equal(journal.latest?.payload, JSON.stringify({ body: "still safe" }));
  assert.equal(journal.nextSequence, 8);
  assert.equal(journal.writeIndex, 1);
});

test("two corrupt slots fail closed instead of returning an empty notebook", () => {
  assert.throws(
    () => inspectJournal(["partial", "also partial"], validate),
    /incomplete local save/,
  );
});

test("the oldest valid slot is replaced after both writes complete", () => {
  const first = encodeEnvelope(10, JSON.stringify({ body: "first" }));
  const second = encodeEnvelope(11, JSON.stringify({ body: "second" }));

  const journal = inspectJournal([first, second], validate);
  assert.equal(journal.latest?.sequence, 11);
  assert.equal(journal.nextSequence, 12);
  assert.equal(journal.writeIndex, 0);
});
