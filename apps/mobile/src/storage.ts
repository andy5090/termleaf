import { Directory, File, Paths } from "expo-file-system";
import { parseLibrary, type Storage } from "@termleaf/notebook";

import { decodeEnvelope, encodeEnvelope, inspectJournal } from "./storageEnvelope.ts";

const notebookDirectory = new Directory(Paths.document, "termleaf");
const slotFiles = [
  new File(notebookDirectory, "notebook-a.json"),
  new File(notebookDirectory, "notebook-b.json"),
] as const;

function ensureDirectory(): void {
  notebookDirectory.create({ idempotent: true, intermediates: true });
}

async function readSlot(file: File): Promise<string | null> {
  if (!file.exists) return null;
  return file.text();
}

async function inspectFiles() {
  ensureDirectory();
  const values = await Promise.all(slotFiles.map(readSlot));
  return inspectJournal(values, (payload) => {
    parseLibrary(payload);
  });
}

export const mobileStorage: Storage = {
  async read() {
    return (await inspectFiles()).latest?.payload ?? null;
  },

  async write(contents) {
    parseLibrary(contents);
    const { nextSequence, writeIndex } = await inspectFiles();
    const target = slotFiles[writeIndex]!;

    target.create({ intermediates: true, overwrite: true });
    target.write(encodeEnvelope(nextSequence, contents));

    try {
      const verified = decodeEnvelope(await target.text(), (payload) => {
        parseLibrary(payload);
      });
      if (verified.sequence === nextSequence && verified.payload === contents) return;
    } catch {
      // The older journal slot remains valid and is used when the app reopens.
    }
    throw new Error(
      "Termleaf could not verify the local save. Your edits remain open; please retry.",
    );
  },
};
