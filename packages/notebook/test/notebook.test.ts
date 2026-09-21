import assert from 'node:assert/strict';
import test from 'node:test';
import { Notebook, parseLibrary, serializeLibrary, type Storage } from '../src/index.ts';

function memory(initial: string | null = null) {
  let value = initial;
  const storage: Storage = {
    async read() { return value; },
    async write(next) { value = next; },
  };
  return { storage, value: () => value };
}

test('multiple Unicode drafts and selected document survive a restart', async () => {
  const disk = memory();
  const notebook = new Notebook(disk.storage);
  await notebook.load();
  notebook.createDocument();
  const first = notebook.getSnapshot().library.activeId!;
  notebook.updateDocument({ title: '첫 원고', body: '한글 日本語 👩‍💻\n\n끝\n' });
  notebook.createDocument();
  notebook.updateDocument({ title: 'Second', body: '' });
  notebook.selectDocument(first);
  assert.equal(await notebook.save(), true);
  const reopened = new Notebook(disk.storage);
  await reopened.load();
  assert.deepEqual(reopened.getSnapshot().library, notebook.getSnapshot().library);
  assert.equal(reopened.getSnapshot().library.documents[0].body, '한글 日本語 👩‍💻\n\n끝\n');
  assert.equal(reopened.getSnapshot().library.activeId, first);
});

test('malformed and incompatible storage is rejected without overwriting', async () => {
  for (const raw of ['{', '{}', '{"version":2,"documents":[]}', '{"version":1,"documents":[],"activeId":"missing"}']) {
    const disk = memory(raw);
    const notebook = new Notebook(disk.storage);
    await notebook.load();
    assert.equal(notebook.getSnapshot().status, 'error');
    assert.equal(notebook.getSnapshot().ready, false);
    notebook.createDocument();
    notebook.updateDocument({ body: 'replacement' });
    assert.equal(await notebook.save(), false);
    assert.equal(disk.value(), raw);
  }
});

test('duplicate ids and malformed document values fail schema validation', () => {
  const draft = { id: 'a', title: '', body: 'text', updatedAt: new Date().toISOString() };
  for (const documents of [[draft, draft], [{ ...draft, body: 4 }], [{ ...draft, updatedAt: 'invalid' }]]) {
    assert.throws(() => parseLibrary(JSON.stringify({ version: 1, activeId: 'a', documents })));
  }
});

test('slow writes are serialized and only the latest content is marked saved', async () => {
  const writes: string[] = [];
  const releases: (() => void)[] = [];
  const notebook = new Notebook({
    async read() { return null; },
    async write(value) { writes.push(value); await new Promise<void>(resolve => releases.push(resolve)); },
  });
  await notebook.load();
  notebook.createDocument();
  notebook.updateDocument({ body: 'old' });
  const saving = notebook.save();
  notebook.updateDocument({ body: 'latest' });
  assert.equal(writes.length, 1);
  assert.equal(notebook.getSnapshot().status, 'saving');
  releases.shift()!();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(writes.length, 2);
  assert.equal(notebook.getSnapshot().status, 'saving');
  releases.shift()!();
  assert.equal(await saving, true);
  assert.equal(notebook.getSnapshot().status, 'saved');
  assert.equal(parseLibrary(writes[1]).documents[0].body, 'latest');
});

test('save failure retains changes and retry saves latest content', async () => {
  let fail = true;
  let stored = '';
  const notebook = new Notebook({
    async read() { return null; },
    async write(value) { if (fail) throw new Error('Disk full'); stored = value; },
  });
  await notebook.load();
  notebook.createDocument();
  notebook.updateDocument({ body: 'Keep this draft' });
  assert.equal(await notebook.save(), false);
  assert.equal(notebook.getSnapshot().status, 'error');
  assert.equal(notebook.getSnapshot().library.documents[0].body, 'Keep this draft');
  fail = false;
  await notebook.retry();
  assert.equal(notebook.getSnapshot().status, 'saved');
  assert.equal(parseLibrary(stored).documents[0].body, 'Keep this draft');
});

test('load retry never starts a blank library over an unreadable one', async () => {
  let fail = true;
  const original = serializeLibrary({ version: 1, activeId: 'a', documents: [{ id: 'a', title: 'Existing', body: 'Keep', updatedAt: new Date().toISOString() }] });
  const notebook = new Notebook({
    async read() { if (fail) throw new Error('Unavailable'); return original; },
    async write() { assert.fail('Loading must not write'); },
  });
  await notebook.load();
  notebook.createDocument();
  assert.equal(notebook.getSnapshot().library.documents.length, 0);
  fail = false;
  await notebook.retry();
  assert.equal(notebook.getSnapshot().library.documents[0].body, 'Keep');
});

test('autosave persists without an explicit save action', async () => {
  const disk = memory();
  const notebook = new Notebook(disk.storage, 1);
  await notebook.load();
  notebook.createDocument();
  notebook.updateDocument({ body: 'Automatic' });
  await new Promise(resolve => setTimeout(resolve, 30));
  assert.equal(parseLibrary(disk.value()!).documents[0].body, 'Automatic');
  assert.equal(notebook.getSnapshot().status, 'saved');
});

test('a clean save followed immediately by an edit and save persists the edit', async () => {
  const disk = memory();
  const notebook = new Notebook(disk.storage);
  await notebook.load();
  notebook.createDocument();
  notebook.updateDocument({ body: 'old' });
  await notebook.save();
  const cleanSave = notebook.save();
  notebook.updateDocument({ body: 'latest' });
  const latestSave = notebook.save();
  assert.deepEqual(await Promise.all([cleanSave, latestSave]), [true, true]);
  assert.equal(parseLibrary(disk.value()!).documents[0].body, 'latest');
  assert.equal(notebook.getSnapshot().status, 'saved');
});

test('runtime patches cannot change identity or persist non-string text', async () => {
  const disk = memory();
  const notebook = new Notebook(disk.storage);
  await notebook.load();
  notebook.createDocument();
  const id = notebook.getSnapshot().library.activeId;
  const extraFields = { body: 'valid', id: 'changed', updatedAt: 'invalid' };
  notebook.updateDocument(extraFields);
  // Exercise the JavaScript caller boundary, where TypeScript cannot help.
  notebook.updateDocument({ body: 42 } as unknown as { body: string });
  assert.equal(await notebook.save(), true);
  const saved = parseLibrary(disk.value()!);
  assert.equal(saved.activeId, id);
  assert.equal(saved.documents[0].id, id);
  assert.equal(saved.documents[0].body, 'valid');
});

test('cloud import keeps unsaved local drafts and their IDs intact', async () => {
  let saved: string | null = null;
  const notebook = new Notebook({ read: async () => null, write: async value => { saved = value; } });
  assert.equal(notebook.importDocument('not ready', 'ignored'), null);
  await notebook.load();
  notebook.createDocument();
  const first = notebook.getSnapshot().library.activeId;
  notebook.updateDocument({ body: 'local unsaved 🌿' });
  const imported = notebook.importDocument('cloud.md', 'remote 日本語');
  assert.notEqual(imported, first);
  assert.deepEqual(notebook.getSnapshot().library.documents.map(d => d.body), ['local unsaved 🌿', 'remote 日本語']);
  assert.equal(await notebook.save(), true);
  assert.equal(JSON.parse(saved!).documents.length, 2);
});
