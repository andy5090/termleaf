import assert from 'node:assert/strict';
import test from 'node:test';
import { CloudClient, CloudError, CloudWorkspace, serverUrl, type FileContent, type GoogleFlow } from '../src/index.ts';

const account = { user: { id: 'user-1', email: 'writer@example.test' }, device: { id: 'device-1', name: 'Laptop' }, subscription: { plan: 'sync', status: 'active', quotaBytes: 10000, usedBytes: 0, currentPeriodEnd: null } };
const makeFile = (version = 1): FileContent => ({ id: 'file-1', name: '문서.md', content: '한글 🌿', version, deleted: false, size: 11, updatedAt: new Date().toISOString() });
const json = (data: unknown, status = 200) => new Response(JSON.stringify(data), { status, headers: { 'content-type': 'application/json' } });

test('default browser transport keeps the Window receiver required by WebKit', async () => {
  const original = globalThis.fetch;
  globalThis.fetch = async function () {
    assert.equal(this, globalThis);
    return json(account);
  };
  try { assert.deepEqual(await new CloudClient('https://sync.example.test').account(), account); }
  finally { globalThis.fetch = original; }
});

const googleFlow = (): GoogleFlow => ({ flowId: 'flow-one', pollToken: 's'.repeat(43), authorizationUrl: 'https://sync.example.test/v1/auth/google/browser?flowId=flow-one&state=public-state', pairingCode: '123456', expiresAt: new Date(Date.now() + 300_000).toISOString() });

test('Google flow secrets stay in POST bodies; only the configured service browser path can open', async () => {
  const requests: { url: string; init?: RequestInit }[] = [];
  const flow = googleFlow();
  const client = new CloudClient('https://sync.example.test', async (url, init) => {
    requests.push({ url: String(url), init });
    if (String(url).endsWith('/start')) return json(flow);
    if (String(url).endsWith('/complete')) return json({ ...account, accessToken: 'session-token', expiresAt: flow.expiresAt });
    return json(account);
  });
  assert.deepEqual(await client.startGoogle('Laptop'), flow);
  assert.deepEqual(await client.waitForGoogle(flow, new AbortController().signal), account);
  await client.account();
  assert.equal(JSON.parse(String(requests[1]!.init!.body)).pollToken, flow.pollToken);
  assert.equal(requests.some(r => r.url.includes(flow.pollToken)), false);
  assert.equal(new Headers(requests[2]!.init!.headers).get('Authorization'), 'Bearer session-token');
  for (const authorizationUrl of ['https://attacker.test/v1/auth/google/browser', 'https://sync.example.test/other', 'https://user@sync.example.test/v1/auth/google/browser', 'https://sync.example.test/v1/auth/google/browser#secret']) {
    const bad = new CloudClient('https://sync.example.test', async () => json({ ...flow, authorizationUrl }));
    await assert.rejects(bad.startGoogle('Laptop'), /invalid response/);
  }
});

test('Google pending, expiry and cancellation do not create a local session', async () => {
  let calls = 0;
  const client = new CloudClient('https://sync.example.test', async () => { calls++; return json({ status: 'pending' }, 202); });
  const flow = googleFlow();
  assert.equal(await client.completeGoogle(flow), null);
  const controller = new AbortController(); controller.abort();
  await assert.rejects(client.waitForGoogle(flow, controller.signal), /canceled/);
  await assert.rejects(client.waitForGoogle({ ...flow, expiresAt: new Date(0).toISOString() }, new AbortController().signal), /expired/);
  assert.equal(calls, 1);
});

test('cancel racing a successful Google handoff revokes the new session', async () => {
  const controller = new AbortController();
  const requests: string[] = [];
  const client = new CloudClient('https://sync.example.test', async url => {
    requests.push(String(url));
    if (String(url).endsWith('/complete')) { controller.abort(); return json({ ...account, accessToken: 'cancel-this-session', expiresAt: googleFlow().expiresAt }); }
    return json({ ok: true });
  });
  await assert.rejects(client.waitForGoogle(googleFlow(), controller.signal), /canceled/);
  assert.equal(requests[1], 'https://sync.example.test/v1/auth/logout');
});

test('credentials go only to a validated HTTPS or loopback server, redirects fail closed', async () => {
  for (const value of ['http://example.com', 'file:///tmp/x', 'https://user:password@example.com', 'https://host.test?q=token', 'https://host.test#token']) assert.throws(() => serverUrl(value));
  assert.equal(serverUrl('http://localhost:8787/'), 'http://localhost:8787');
  const requests: RequestInit[] = [];
  const client = new CloudClient('https://sync.example.test', async (_url, init) => {
    requests.push(init!);
    return json(requests.length === 1 ? { ...account, accessToken: 'private-token', expiresAt: '2030-01-01' } : account);
  });
  await client.authenticate('login', account.user.email, 'private password', 'Laptop');
  await client.account();
  assert.equal(requests[0]!.redirect, 'error');
  assert.equal(new Headers(requests[0]!.headers).get('Authorization'), null);
  assert.equal(new Headers(requests[1]!.headers).get('Authorization'), 'Bearer private-token');
});

test('file pagination consumes tombstones and rejects stalled cursors', async () => {
  let call = 0;
  const client = new CloudClient('https://sync.example.test', async () => json(++call === 1
    ? { files: [makeFile()], cursor: 1, hasMore: true }
    : { files: [{ ...makeFile(2), deleted: true, size: 0 }], cursor: 2, hasMore: false }));
  assert.deepEqual(await client.files(), []);
  const broken = new CloudClient('https://sync.example.test', async () => json({ files: [], cursor: 0, hasMore: true }));
  await assert.rejects(broken.files(), /invalid response/);
});

test('stale writes retain the base revision and local content; there is no retry overwrite', async () => {
  const calls: { method?: string; body: unknown }[] = [];
  const client = new CloudClient('https://sync.example.test', async (_url, init) => {
    calls.push({ method: init?.method, body: init?.body ? JSON.parse(String(init.body)) : null });
    return init?.method === 'POST' ? json(makeFile()) : json({ error: { code: 'version_conflict', message: 'conflict' } }, 409);
  });
  const workspace = new CloudWorkspace(client);
  await workspace.upload('local-one', '문서.md', '한글 🌿');
  await assert.rejects(workspace.upload('local-one', '문서.md', 'offline edits'), (error: unknown) => error instanceof CloudError && error.status === 409);
  await assert.rejects(workspace.upload('local-one', '문서.md', 'offline edits'));
  assert.deepEqual(calls.slice(1).map(call => call.body), [
    { name: '문서.md', content: 'offline edits', baseVersion: 1 },
    { name: '문서.md', content: 'offline edits', baseVersion: 1 },
  ]);
});

test('downloads import a separate copy and bind only after local import succeeds', async () => {
  const methods: string[] = [];
  const client = new CloudClient('https://sync.example.test', async (_url, init) => { methods.push(init!.method!); return json(makeFile(3)); });
  const workspace = new CloudWorkspace(client);
  await assert.rejects(workspace.open('file-1', async () => { throw new Error('disk full'); }), /disk full/);
  await workspace.upload('new-local', 'copy.md', 'retained local');
  assert.equal(methods.at(-1), 'POST');
  await workspace.open('file-1', async file => { assert.equal(file.content, '한글 🌿'); return 'imported'; });
  await workspace.upload('imported', 'copy.md', 'updated');
  assert.equal(methods.at(-1), 'PUT');
});

test('a different account workspace never reuses previous file bindings', async () => {
  let posts = 0;
  const client = new CloudClient('https://sync.example.test', async (_url, init) => { assert.equal(init?.method, 'POST'); posts++; return json(makeFile()); });
  await new CloudWorkspace(client).upload('same-local-id', 'one', 'one');
  await new CloudWorkspace(client).upload('same-local-id', 'two', 'two');
  assert.equal(posts, 2);
});

test('invalid files and hostile billing redirects are rejected', async () => {
  const invalid = new CloudClient('https://sync.example.test', async () => json({ ...makeFile(), content: null }));
  await assert.rejects(invalid.read('file-1'), /invalid response/);
  const hostile = new CloudClient('https://sync.example.test', async () => json({ url: 'https://polar.sh.evil.test/checkout' }));
  await assert.rejects(hostile.billing('checkout'), /invalid response/);
});
