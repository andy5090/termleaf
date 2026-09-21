import { useEffect, useMemo, useSyncExternalStore } from 'react';
import { Notebook, type Storage } from './index';

export function useNotebook(storage: Storage) {
  const notebook = useMemo(() => new Notebook(storage), [storage]);
  const snapshot = useSyncExternalStore(notebook.subscribe, notebook.getSnapshot, notebook.getSnapshot);
  useEffect(() => { void notebook.load(); }, [notebook]);
  return {
    ...snapshot,
    documents: snapshot.library.documents,
    activeDocument: snapshot.library.documents.find(draft => draft.id === snapshot.library.activeId) ?? null,
    createDocument: notebook.createDocument,
    importDocument: notebook.importDocument,
    selectDocument: notebook.selectDocument,
    updateDocument: notebook.updateDocument,
    save: notebook.save,
    retry: notebook.retry,
  };
}
