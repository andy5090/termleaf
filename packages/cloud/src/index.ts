export interface Subscription {
  plan: string; status: string; quotaBytes: number; usedBytes: number; currentPeriodEnd: string | null;
}
export interface Account {
  user: { id: string; email: string };
  device: { id: string; name: string };
  subscription: Subscription;
}
export interface Session extends Account { accessToken: string; expiresAt: string }
export interface GoogleFlow { flowId: string; pollToken: string; authorizationUrl: string; expiresAt: string; pairingCode: string }
export interface CloudFile {
  id: string; name: string; version: number; deleted: boolean; size: number; updatedAt: string;
}
export interface FileContent extends CloudFile { content: string | null }
export interface FilePage { files: CloudFile[]; cursor: number; hasMore: boolean }
export interface Device { id: string; name: string }
export class CloudError extends Error {
  readonly status: number;
  readonly code: string;
  constructor(status: number, code: string, message: string) {
    super(message); this.name = 'CloudError'; this.status = status; this.code = code;
  }
}

export function serverUrl(value: string): string {
  const url = new URL(value.trim());
  const loopback = ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname);
  if ((url.protocol !== 'https:' && !(url.protocol === 'http:' && loopback)) || url.username || url.password || url.search || url.hash) {
    throw new Error('Use an HTTPS server URL (HTTP is allowed only for localhost development).');
  }
  return url.href.replace(/\/+$/, '');
}

function record(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}
function invalid(): never { throw new CloudError(502, 'invalid_response', 'The storage server returned an invalid response.'); }
function file(value: unknown, withContent = false): FileContent {
  if (!record(value) || typeof value.id !== 'string' || !/^[a-zA-Z0-9_-]{1,128}$/.test(value.id)
    || typeof value.name !== 'string' || !Number.isSafeInteger(value.version) || Number(value.version) < 1
    || typeof value.deleted !== 'boolean' || !Number.isSafeInteger(value.size) || Number(value.size) < 0
    || typeof value.updatedAt !== 'string'
    || (withContent && (value.deleted ? value.content !== null : typeof value.content !== 'string'))) invalid();
  return value as unknown as FileContent;
}
function account(value: unknown): Account {
  if (!record(value) || !record(value.user) || typeof value.user.id !== 'string' || typeof value.user.email !== 'string'
    || !record(value.device) || typeof value.device.id !== 'string' || typeof value.device.name !== 'string'
    || !record(value.subscription) || typeof value.subscription.plan !== 'string' || typeof value.subscription.status !== 'string'
    || !Number.isSafeInteger(value.subscription.quotaBytes) || Number(value.subscription.quotaBytes) < 0
    || !Number.isSafeInteger(value.subscription.usedBytes) || Number(value.subscription.usedBytes) < 0
    || (value.subscription.currentPeriodEnd !== null && typeof value.subscription.currentPeriodEnd !== 'string')) invalid();
  const result = value as unknown as Account;
  return { user: result.user, device: result.device, subscription: result.subscription };
}

/** Tokens stay in memory. No automatic retries of mutating requests. */
export class CloudClient {
  readonly url: string;
  private token: string | null = null;
  private transport: typeof fetch;
  constructor(url: string, transport: typeof fetch = globalThis.fetch.bind(globalThis)) { this.url = serverUrl(url); this.transport = transport; }
  private async request(path: string, method = 'GET', body?: unknown, signal?: AbortSignal): Promise<unknown> {
    const abort = new AbortController();
    const cancel = () => abort.abort();
    if (signal?.aborted) abort.abort();
    signal?.addEventListener('abort', cancel, { once: true });
    const timer = setTimeout(() => abort.abort(), 30_000);
    try {
      const response = await this.transport(`${this.url}/v1${path}`, {
        method, signal: abort.signal, redirect: 'error',
        headers: { Accept: 'application/json', ...(body === undefined ? {} : { 'Content-Type': 'application/json' }), ...(this.token ? { Authorization: `Bearer ${this.token}` } : {}) },
        ...(body === undefined ? {} : { body: JSON.stringify(body) }),
      });
      const value: unknown = await response.json().catch(() => invalid());
      if (!response.ok) {
        const error = record(value) && record(value.error) ? value.error : null;
        throw new CloudError(response.status, typeof error?.code === 'string' ? error.code : 'request_failed',
          response.status === 409 ? 'This file changed on another device. Open its latest copy before merging; your local document is unchanged.'
            : typeof error?.message === 'string' ? error.message : `Storage request failed (${response.status}).`);
      }
      return value;
    } finally { clearTimeout(timer); signal?.removeEventListener('abort', cancel); }
  }
  private acceptSession(value: unknown): Account {
    const result = account(value);
    if (!record(value) || typeof value.accessToken !== 'string' || !value.accessToken || typeof value.expiresAt !== 'string') invalid();
    this.token = value.accessToken;
    return result;
  }
  /** Local development compatibility; consumer apps use Google sign-in. */
  async authenticate(kind: 'login' | 'register', email: string, password: string, deviceName: string): Promise<Account> {
    return this.acceptSession(await this.request(`/auth/${kind}`, 'POST', { email, password, deviceName }));
  }
  async startGoogle(deviceName: string, signal?: AbortSignal): Promise<GoogleFlow> {
    const value = await this.request('/auth/google/start', 'POST', { deviceName }, signal);
    if (!record(value) || typeof value.flowId !== 'string' || !value.flowId || typeof value.pollToken !== 'string'
      || value.pollToken.length < 32 || typeof value.authorizationUrl !== 'string' || typeof value.expiresAt !== 'string'
      || !Number.isFinite(Date.parse(value.expiresAt)) || typeof value.pairingCode !== 'string' || !/^\d{6}$/.test(value.pairingCode)) invalid();
    const url = new URL(value.authorizationUrl);
    if (url.origin !== new URL(this.url).origin || url.pathname !== '/v1/auth/google/browser' || url.username || url.password || url.hash) invalid();
    return value as unknown as GoogleFlow;
  }
  async completeGoogle(flow: GoogleFlow, signal?: AbortSignal): Promise<Account | null> {
    const value = await this.request('/auth/google/complete', 'POST', { flowId: flow.flowId, pollToken: flow.pollToken }, signal);
    if (record(value) && value.status === 'pending') return null;
    return this.acceptSession(value);
  }
  async cancelGoogle(flow: GoogleFlow): Promise<void> {
    await this.request('/auth/google/cancel', 'POST', { flowId: flow.flowId, pollToken: flow.pollToken });
  }
  async waitForGoogle(flow: GoogleFlow, signal: AbortSignal): Promise<Account> {
    const deadline = Math.min(Date.parse(flow.expiresAt), Date.now() + 5 * 60_000);
    while (!signal.aborted && Date.now() < deadline) {
      const result = await this.completeGoogle(flow, signal);
      if (result) {
        // A cancellation can race the successful response; revoke that session.
        if (signal.aborted) { await this.logout(); break; }
        return result;
      }
      await new Promise<void>(resolve => {
        const finish = () => { clearTimeout(timer); signal.removeEventListener('abort', finish); resolve(); };
        const timer = setTimeout(finish, 2000);
        signal.addEventListener('abort', finish, { once: true });
        if (signal.aborted) finish();
      });
    }
    throw new Error(signal.aborted ? 'Sign-in canceled.' : 'Sign-in expired. Please try again.');
  }
  async account(): Promise<Account> { return account(await this.request('/account')); }
  async logout(): Promise<void> { try { await this.request('/auth/logout', 'POST', {}); } finally { this.token = null; } }
  async list(after = 0): Promise<FilePage> {
    const value = await this.request(`/files?after=${after}&limit=100`);
    if (!record(value) || !Array.isArray(value.files) || !Number.isSafeInteger(value.cursor) || Number(value.cursor) < after || typeof value.hasMore !== 'boolean') invalid();
    return { files: value.files.map(item => file(item)), cursor: Number(value.cursor), hasMore: value.hasMore };
  }
  async files(): Promise<CloudFile[]> {
    const files = new Map<string, CloudFile>();
    let cursor = 0;
    for (let page = 0; page < 1000; page++) {
      const result = await this.list(cursor);
      for (const item of result.files) { if (item.deleted) files.delete(item.id); else files.set(item.id, item); }
      if (!result.hasMore) return [...files.values()];
      if (result.cursor <= cursor) invalid();
      cursor = result.cursor;
    }
    throw new Error('Too many file changes. Please refresh again.');
  }
  async read(id: string): Promise<FileContent> { return file(await this.request(`/files/${encodeURIComponent(id)}`), true); }
  async create(name: string, content: string): Promise<FileContent> { return file(await this.request('/files', 'POST', { name, content }), true); }
  async write(id: string, name: string, content: string, baseVersion: number): Promise<FileContent> {
    return file(await this.request(`/files/${encodeURIComponent(id)}`, 'PUT', { name, content, baseVersion }), true);
  }
  async remove(id: string, baseVersion: number): Promise<FileContent> { return file(await this.request(`/files/${encodeURIComponent(id)}`, 'DELETE', { baseVersion }), true); }
  async billing(kind: 'checkout' | 'portal'): Promise<string> {
    const value = await this.request(`/billing/${kind}`, 'POST', {});
    if (!record(value) || typeof value.url !== 'string') invalid();
    const url = new URL(value.url);
    if (url.protocol !== 'https:' || url.username || url.password || !(url.hostname === 'polar.sh' || url.hostname.endsWith('.polar.sh'))) invalid();
    return url.href;
  }
  async devices(): Promise<Device[]> {
    const value = await this.request('/devices');
    if (!record(value) || !Array.isArray(value.devices) || !value.devices.every(d => record(d) && typeof d.id === 'string' && typeof d.name === 'string')) invalid();
    return value.devices as Device[];
  }
  async revoke(id: string): Promise<void> { await this.request(`/devices/${encodeURIComponent(id)}`, 'DELETE'); }
}

/** Bindings are scoped to one authenticated account and survive panel opens, not app restarts. */
export class CloudWorkspace {
  readonly client: CloudClient;
  private bindings = new Map<string, { id: string; version: number }>();
  constructor(client: CloudClient) { this.client = client; }
  async upload(localId: string, name: string, content: string): Promise<FileContent> {
    const binding = this.bindings.get(localId);
    const result = binding ? await this.client.write(binding.id, name, content, binding.version) : await this.client.create(name, content);
    this.bindings.set(localId, { id: result.id, version: result.version });
    return result;
  }
  async open(id: string, importCopy: (file: FileContent) => Promise<string>): Promise<FileContent> {
    const result = await this.client.read(id);
    if (result.deleted || result.content === null) throw new Error('This cloud file was deleted.');
    const localId = await importCopy(result);
    this.bindings.set(localId, { id: result.id, version: result.version });
    return result;
  }
}
