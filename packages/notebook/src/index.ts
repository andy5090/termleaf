export interface Draft {
  readonly id: string;
  readonly title: string;
  readonly body: string;
  readonly updatedAt: string;
}

export interface Library {
  readonly version: 1;
  readonly activeId: string | null;
  readonly documents: readonly Draft[];
}

export interface Storage {
  read(): Promise<string | null>;
  /** Resolve only after the platform storage operation has completed. */
  write(contents: string): Promise<void>;
}

export interface Snapshot {
  readonly library: Library;
  readonly ready: boolean;
  readonly status: 'loading' | 'saving' | 'saved' | 'error';
  readonly error: string | null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/** Reject unknown formats and corruption instead of replacing the user's data. */
export function parseLibrary(contents: string): Library {
  const value: unknown = JSON.parse(contents);
  const invalid = () => new Error('The saved notebook could not be read. Your stored files have not been changed.');
  if (!isRecord(value) || value.version !== 1 || !Array.isArray(value.documents)) throw invalid();
  const ids = new Set<string>();
  for (const draft of value.documents) {
    if (!isRecord(draft) || typeof draft.id !== 'string' || !draft.id || ids.has(draft.id)
      || typeof draft.title !== 'string' || typeof draft.body !== 'string'
      || typeof draft.updatedAt !== 'string' || !Number.isFinite(Date.parse(draft.updatedAt))) throw invalid();
    ids.add(draft.id);
  }
  if ((value.documents.length === 0 && value.activeId !== null)
    || (value.documents.length > 0 && (typeof value.activeId !== 'string' || !ids.has(value.activeId)))) throw invalid();
  return value as unknown as Library;
}

export function serializeLibrary(library: Library): string {
  return JSON.stringify(library);
}

let documentSequence = 0;

/** One session owns a notebook. All storage writes are serialized. */
export class Notebook {
  private snapshot: Snapshot = {
    library: { version: 1, activeId: null, documents: [] },
    ready: false,
    status: 'loading',
    error: null,
  };
  private listeners = new Set<() => void>();
  private loading: Promise<void> | null = null;
  private writing: Promise<boolean> | null = null;
  private timer: ReturnType<typeof setTimeout> | undefined;
  private revision = 0;
  private storedRevision = 0;
  private storage: Storage;
  private autosaveDelay: number;

  constructor(storage: Storage, autosaveDelay = 300) {
    this.storage = storage;
    this.autosaveDelay = autosaveDelay;
  }

  getSnapshot = (): Snapshot => this.snapshot;

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => { this.listeners.delete(listener); };
  };

  private publish(patch: Partial<Snapshot>): void {
    this.snapshot = { ...this.snapshot, ...patch };
    this.listeners.forEach(listener => listener());
  }

  load = (): Promise<void> => {
    if (this.snapshot.ready) return Promise.resolve();
    if (this.loading) return this.loading;
    this.publish({ status: 'loading', error: null });
    this.loading = this.read().finally(() => { this.loading = null; });
    return this.loading;
  };

  private async read(): Promise<void> {
    try {
      const contents = await this.storage.read();
      const library = contents === null ? this.snapshot.library : parseLibrary(contents);
      this.publish({ library, ready: true, status: 'saved', error: null });
    } catch (error) {
      this.publish({ status: 'error', error: this.message(error) });
    }
  }

  private change(library: Library): void {
    if (!this.snapshot.ready) return;
    this.revision += 1;
    this.publish({ library, status: 'saving', error: null });
    clearTimeout(this.timer);
    this.timer = setTimeout(() => { void this.save(); }, this.autosaveDelay);
  }

  createDocument = (): void => {
    if (!this.snapshot.ready) return;
    const draft: Draft = {
      id: `${Date.now().toString(36)}-${++documentSequence}-${Math.random().toString(36).slice(2)}`,
      title: '', body: '', updatedAt: new Date().toISOString(),
    };
    this.change({ ...this.snapshot.library, activeId: draft.id, documents: [...this.snapshot.library.documents, draft] });
  };

  selectDocument = (id: string): void => {
    const library = this.snapshot.library;
    if (library.activeId !== id && library.documents.some(draft => draft.id === id)) this.change({ ...library, activeId: id });
  };

  updateDocument = (patch: { title?: string; body?: string }): void => {
    if (!isRecord(patch)
      || (patch.title !== undefined && typeof patch.title !== 'string')
      || (patch.body !== undefined && typeof patch.body !== 'string')) return;
    const library = this.snapshot.library;
    const active = library.documents.find(draft => draft.id === library.activeId);
    if (!active || (patch.title === undefined || patch.title === active.title)
      && (patch.body === undefined || patch.body === active.body)) return;
    const draft = { ...active, title: patch.title ?? active.title, body: patch.body ?? active.body, updatedAt: new Date().toISOString() };
    this.change({ ...library, documents: library.documents.map(item => item.id === active.id ? draft : item) });
  };

  save = async (): Promise<boolean> => {
    clearTimeout(this.timer);
    if (!this.snapshot.ready) return false;
    while (this.storedRevision < this.revision) {
      if (!this.writing) this.writing = this.drain().finally(() => { this.writing = null; });
      if (!await this.writing) return false;
    }
    return true;
  };

  private async drain(): Promise<boolean> {
    try {
      while (this.storedRevision < this.revision) {
        const revision = this.revision;
        const contents = serializeLibrary(this.snapshot.library);
        this.publish({ status: 'saving', error: null });
        await this.storage.write(contents);
        this.storedRevision = revision;
      }
      this.publish({ status: 'saved', error: null });
      return true;
    } catch (error) {
      clearTimeout(this.timer);
      this.publish({ status: 'error', error: this.message(error) });
      return false;
    }
  }

  retry = async (): Promise<void> => {
    if (!this.snapshot.ready) await this.load();
    else await this.save();
  };

  private message(error: unknown): string {
    return error instanceof Error ? error.message : typeof error === 'string' ? error : 'Storage is unavailable. Your changes are still open; retry saving.';
  }
}
