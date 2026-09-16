import assert from "node:assert/strict";
import { test } from "node:test";
import { registerHooks } from "node:module";
import { Notebook, serializeLibrary } from "@termleaf/notebook";
import { encodeEnvelope } from "./storageEnvelope.ts";

test("a transient read error cannot replace a newer snapshot with an older one", async () => {
  const library = (body: string) => serializeLibrary({
    version: 1, activeId: "draft", documents: [{
      id: "draft", title: "Stored", body, updatedAt: "2026-09-14T00:00:00Z",
    }],
  });
  const slots = new Map([
    ["notebook-a.json", encodeEnvelope(1, library("older"))],
    ["notebook-b.json", encodeEnvelope(2, library("newer"))],
  ]);
  let unavailable = true;
  let writes = 0;
  class FakeFile {
    name: string;
    constructor(_directory: unknown, name: string) { this.name = name; }
    get exists() { return slots.has(this.name); }
    async text() {
      if (unavailable && this.name === "notebook-b.json") throw new Error("Temporarily unavailable");
      return slots.get(this.name)!;
    }
    create() { /* The fake directory and slots already exist. */ }
    write(contents: string) { writes++; slots.set(this.name, contents); }
  }
  const fixtureKey = Symbol.for("termleaf.test.filesystem");
  Object.defineProperty(globalThis, fixtureKey, {
    value: { File: FakeFile, Directory: class { create() {} }, Paths: { document: "/test" } },
    configurable: true,
  });
  const filesystemUrl = import.meta.resolve("expo-file-system");
  // Expo ships TypeScript for a native runtime. Substitute only its I/O module.
  const hooks = registerHooks({
    load(url, context, nextLoad) {
      if (url !== filesystemUrl) return nextLoad(url, context);
      return {
        format: "module", shortCircuit: true,
        source: `const fs = globalThis[Symbol.for("termleaf.test.filesystem")];
          export const File = fs.File;
          export const Directory = fs.Directory;
          export const Paths = fs.Paths;`,
      };
    },
  });
  try {
    const { mobileStorage } = await import("./storage.ts");
    const notebook = new Notebook(mobileStorage);
    await notebook.load();
    assert.equal(notebook.getSnapshot().ready, false);
    assert.match(notebook.getSnapshot().error!, /Temporarily unavailable/);
    notebook.createDocument();
    assert.equal(await notebook.save(), false);
    assert.equal(writes, 0);
    unavailable = false;
    await notebook.retry();
    assert.equal(notebook.getSnapshot().ready, true);
    assert.equal(notebook.getSnapshot().library.documents[0].body, "newer");
    assert.equal(writes, 0);
  } finally {
    hooks.deregister();
    Reflect.deleteProperty(globalThis, fixtureKey);
  }
});
