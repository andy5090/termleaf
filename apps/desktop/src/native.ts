import { invoke } from '@tauri-apps/api/core';

export interface EditorView {
  activeDocumentId: number;
  documents: { id: number; path: string | null; label: string; dirty: boolean }[];
  text: string;
  cursorUtf16: number;
  path: string | null;
  dirty: boolean;
  composing: string;
  words: number;
  characters: number;
  inputMode: 'os' | 'ko' | 'ja';
  japaneseKatakana: boolean;
  candidate: string | null;
  config: {
    theme: string; fontSize: number; lineSpacing: number; pageWidth: boolean;
    bigFont: boolean; focusMode: boolean; sound: boolean; soundProfile: string;
    backspaceSound: boolean; returnSound: boolean; language: string; autosaveSecs: number;
    showWelcome: boolean;
  };
  colors: { fg: string; bg: string; dim: string; accent: string; pixel: string; pixelOff: string };
  glyphs: { width: number; height: number; rows: number[] }[];
  languages: { code: string; name: string; installed: boolean; liveInput: boolean }[];
}

export interface FileLocation {
  input: string;
  candidates: { path: string; name: string; isDir: boolean }[];
}

export const native = {
  importDocument: (text: string) => invoke<EditorView>('editor_import', { text }),
  openBilling: (url: string) => invoke<void>('open_billing_url', { url }),
  openAuthentication: (url: string) => invoke<void>('open_auth_url', { url }),
  newDocument: () => invoke<EditorView>('editor_new'),
  switchDocument: (documentId: number) => invoke<EditorView>('editor_switch', { documentId }),
  snapshot: () => invoke<EditorView>('editor_snapshot'),
  sync: (text: string, cursorUtf16: number) => invoke<EditorView>('editor_sync', { text, cursorUtf16 }),
  select: (cursorUtf16: number) => invoke<EditorView>('editor_select', { cursorUtf16 }),
  key: (key: string, ctrl = false, alt = false, shift = false) => invoke<EditorView>('editor_key', { key, ctrl, alt, shift }),
  action: (name: string) => invoke<EditorView>('editor_action', { name }),
  open: (path: string) => invoke<EditorView>('editor_open', { path }),
  browse: (path: string) => invoke<FileLocation>('editor_browse', { path }),
  save: (path: string | null = null) => invoke<EditorView>('editor_save', { path }),
  language: (code: string, operation: 'select' | 'install' | 'remove') => invoke<EditorView>('editor_language', { code, operation }),
};
