export interface StorageEnvelope {
  version: 1;
  sequence: number;
  length: number;
  checksum: string;
  payload: string;
}

function checksum(value: string): string {
  let hash = 0x811c9dc5;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 0x01000193);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

export function encodeEnvelope(sequence: number, payload: string): string {
  const envelope: StorageEnvelope = {
    version: 1,
    sequence,
    length: payload.length,
    checksum: checksum(payload),
    payload,
  };
  return JSON.stringify(envelope);
}

export function decodeEnvelope(
  value: string,
  validatePayload: (payload: string) => void,
): StorageEnvelope {
  const parsed: unknown = JSON.parse(value);
  if (typeof parsed !== "object" || parsed === null) {
    throw new Error("Invalid Termleaf storage snapshot.");
  }
  const candidate = parsed as Partial<StorageEnvelope>;
  if (
    candidate.version !== 1 ||
    !Number.isSafeInteger(candidate.sequence) ||
    (candidate.sequence ?? 0) < 1 ||
    typeof candidate.payload !== "string" ||
    candidate.length !== candidate.payload.length ||
    candidate.checksum !== checksum(candidate.payload)
  ) {
    throw new Error("Invalid Termleaf storage snapshot.");
  }

  const envelope = candidate as StorageEnvelope;
  validatePayload(envelope.payload);
  return envelope;
}

export interface JournalState {
  latest: StorageEnvelope | null;
  nextSequence: number;
  writeIndex: number;
}

/** Selects the newest intact slot while leaving it untouched for the next write. */
export function inspectJournal(
  slotValues: readonly (string | null)[],
  validatePayload: (payload: string) => void,
): JournalState {
  if (slotValues.length < 2) {
    throw new Error("A durable journal needs at least two slots.");
  }

  const decoded = slotValues.map((value) => {
    if (value === null) return null;
    try {
      return decodeEnvelope(value, validatePayload);
    } catch {
      return null;
    }
  });
  const valid = decoded.filter((value): value is StorageEnvelope => value !== null);

  if (valid.length === 0 && slotValues.some((value) => value !== null)) {
    throw new Error(
      "Termleaf found an incomplete local save. Your stored files have not been changed.",
    );
  }

  const latest = valid.reduce<StorageEnvelope | null>(
    (current, value) =>
      current === null || value.sequence > current.sequence ? value : current,
    null,
  );
  const reusableIndex = decoded.findIndex((value) => value === null);
  const writeIndex =
    reusableIndex >= 0
      ? reusableIndex
      : decoded[0]!.sequence <= decoded[1]!.sequence
        ? 0
        : 1;

  return {
    latest,
    nextSequence: (latest?.sequence ?? 0) + 1,
    writeIndex,
  };
}
