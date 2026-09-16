import {
  type CSSProperties,
  type FormEvent,
  type KeyboardEvent as ReactKeyboardEvent,
  useCallback,
  useEffect,
  useRef,
  useState,
} from 'react';
import { destroyAppWindow, requestAppClose, useCloseProtection } from './closeProtection.ts';
import { IpcQueue } from './ipcQueue.ts';
import { native, type EditorView, type FileLocation } from './native.ts';
import { isNativeDesktop } from './storage.ts';

type Panel = 'welcome' | 'help' | 'language' | 'sound' | null;
type FilePrompt = {
  kind: 'open' | 'save'; path: string; closeAfterSave: boolean;
  candidates: FileLocation['candidates']; browsing: boolean;
} | null;

const actions = {
  input: 'cycle-input', reverseInput: 'reverse-input', script: 'toggle-script',
  focus: 'toggle-focus', big: 'toggle-big', page: 'toggle-page', spacing: 'cycle-spacing',
  theme: 'cycle-theme', smaller: 'font-dec', larger: 'font-inc', sound: 'toggle-sound',
  soundProfile: 'cycle-sound', backspaceSound: 'toggle-backspace-sound',
  returnSound: 'toggle-return-sound',
  welcome: 'toggle-welcome',
} as const;

type ActionName = typeof actions[keyof typeof actions];

const copy = {
  en: {
    newWriting: 'New writing', documents: 'Open documents', expandSidebar: 'Expand sidebar', collapseSidebar: 'Collapse sidebar', unsaved: 'Unsaved', saveAll: 'Save all',
    loading: 'Opening Termleaf…', native: 'Termleaf Desktop requires the native app runtime.',
    nativeHint: 'Run the Tauri desktop application to open and save local documents.',
    untitled: 'untitled', muted: 'muted', sound: 'sound', words: 'words', chars: 'chars',
    line: 'line', page: 'page', compose: 'compose', on: 'on', off: 'off', candidate: 'candidate',
    help: 'Help', input: 'Input', focus: 'Focus', big: 'Big', spacing: 'Spacing', theme: 'Theme',
    size: 'Size', language: 'Language', soundMenu: 'Sound', keys: 'Keys', saveAs: 'Save as',
    open: 'Open', save: 'Save', quit: 'Quit', close: 'Close', fileOpen: 'Open document',
    fileSave: 'Save document as', fileHint: 'Enter a local file path.', cancel: 'Cancel',
    confirmOpen: 'Open', confirmSave: 'Save', helpTitle: 'Termleaf keys', languageTitle: 'Languages',
    soundTitle: 'Typing sound', install: 'Install', remove: 'Remove', use: 'Use', current: 'Current',
    browse: 'Browse', welcomeTitle: 'Welcome to Termleaf', welcomeBody: 'Write with the operating system input method, or press F2 for Termleaf Korean and Japanese input.', dontShow: "Don't show this guide again",
    unsavedTitle: 'Unsaved changes', closeQuestion: 'Save all changed documents before closing Termleaf?', discard: 'Discard',
  },
  ko: {
    newWriting: '새로 글쓰기', documents: '열린 문서', expandSidebar: '사이드바 펼치기', collapseSidebar: '사이드바 접기', unsaved: '저장 안 됨', saveAll: '모두 저장',
    loading: 'Termleaf 여는 중…', native: 'Termleaf Desktop은 네이티브 앱에서 실행됩니다.',
    nativeHint: '로컬 문서를 열고 저장하려면 Tauri 데스크톱 앱을 실행하세요.',
    untitled: '제목 없음', muted: '무음', sound: '소리', words: '단어', chars: '글자',
    line: '줄', page: '종이', compose: '조합', on: '켬', off: '끔', candidate: '후보',
    help: '도움', input: '입력', focus: '집중', big: '큰글', spacing: '간격', theme: '테마',
    size: '크기', language: '언어', soundMenu: '소리', keys: '타자음', saveAs: '다른 이름',
    open: '열기', save: '저장', quit: '종료', close: '닫기', fileOpen: '문서 열기',
    fileSave: '다른 이름으로 저장', fileHint: '로컬 파일 경로를 입력하세요.', cancel: '취소',
    confirmOpen: '열기', confirmSave: '저장', helpTitle: 'Termleaf 단축키', languageTitle: '언어',
    soundTitle: '타자 소리', install: '설치', remove: '제거', use: '사용', current: '현재',
    browse: '찾기', welcomeTitle: 'Termleaf에 오신 것을 환영합니다', welcomeBody: '운영체제 입력기를 사용하거나 F2를 눌러 Termleaf 한글·일본어 입력을 선택하세요.', dontShow: '이 안내를 다시 표시하지 않기',
    unsavedTitle: '저장하지 않은 변경 사항', closeQuestion: 'Termleaf를 닫기 전에 변경한 문서를 모두 저장할까요?', discard: '버리기',
  },
  ja: {
    newWriting: '新しく書く', documents: '開いている文書', expandSidebar: 'サイドバーを開く', collapseSidebar: 'サイドバーを閉じる', unsaved: '未保存', saveAll: 'すべて保存',
    loading: 'Termleaf を開いています…', native: 'Termleaf Desktop はネイティブアプリで動作します。',
    nativeHint: 'ローカル文書を開いて保存するには Tauri デスクトップアプリを起動してください。',
    untitled: '無題', muted: '消音', sound: '音', words: '語', chars: '字',
    line: '行', page: '幅', compose: '入力', on: 'オン', off: 'オフ', candidate: '候補',
    help: 'ヘルプ', input: '入力', focus: '集中', big: '拡大', spacing: '間隔', theme: 'テーマ',
    size: 'サイズ', language: '言語', soundMenu: '音', keys: 'キー音', saveAs: '別名保存',
    open: '開く', save: '保存', quit: '終了', close: '閉じる', fileOpen: '文書を開く',
    fileSave: '名前を付けて保存', fileHint: 'ローカルファイルのパスを入力してください。', cancel: 'キャンセル',
    confirmOpen: '開く', confirmSave: '保存', helpTitle: 'Termleaf キー', languageTitle: '言語',
    soundTitle: 'キー音', install: 'インストール', remove: '削除', use: '使用', current: '使用中',
    browse: '参照', welcomeTitle: 'Termleaf へようこそ', welcomeBody: 'OS の入力メソッドを使うか、F2 で Termleaf の韓国語・日本語入力を選べます。', dontShow: '次回からこのガイドを表示しない',
    unsavedTitle: '未保存の変更', closeQuestion: 'Termleaf を閉じる前に変更した文書をすべて保存しますか？', discard: '破棄',
  },
} as const;

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function fileName(path: string | null, fallback: string): string {
  if (!path) return fallback;
  return path.split(/[\\/]/).pop() || path;
}

function BitmapGlyph({ glyph }: { glyph: EditorView['glyphs'][number] }) {
  const pixels: React.ReactNode[] = [];
  for (let y = 0; y < glyph.height; y += 1) {
    const row = glyph.rows[y] ?? 0;
    for (let x = 0; x < glyph.width; x += 1) {
      const lit = ((row >> (glyph.width - x - 1)) & 1) === 1;
      pixels.push(<rect className={lit ? 'pixel pixel--on' : 'pixel'} key={`${x}:${y}`} x={x} y={y} width="1" height="1" />);
    }
  }
  return <svg className="bitmap-glyph" viewBox={`0 0 ${glyph.width} ${glyph.height}`} aria-hidden="true">{pixels}</svg>;
}

function Overlay({ title, onClose, children }: { title: string; onClose: () => void; children: React.ReactNode }) {
  const dialog = useRef<HTMLElement>(null);
  useEffect(() => {
    const element = dialog.current;
    if (element && !element.contains(document.activeElement)) element.querySelector<HTMLElement>('input, button')?.focus();
    return () => { requestAnimationFrame(() => document.getElementById('native-editor')?.focus()); };
  }, []);
  return (
    <div className="overlay" role="presentation" onMouseDown={event => event.target === event.currentTarget && onClose()}>
      <section ref={dialog} className="dialog" role="dialog" aria-modal="true" aria-label={title} onKeyDown={event => {
        if (event.key !== 'Tab') return;
        const controls = Array.from(dialog.current?.querySelectorAll<HTMLElement>('button:not(:disabled), input:not(:disabled), [tabindex="0"]') ?? []);
        const first = controls[0];
        const last = controls.at(-1);
        if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
        else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
      }}>
        <header className="dialog__header"><strong>{title}</strong><button type="button" onClick={onClose} aria-label="Close">×</button></header>
        {children}
      </section>
    </div>
  );
}

export function App() {
  const queue = useRef(new IpcQueue());
  const alive = useRef(true);
  const textarea = useRef<HTMLTextAreaElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const textRef = useRef('');
  const cursorRef = useRef(0);
  const requestRef = useRef(0);
  const lastSelectedRef = useRef(0);
  const pendingKeysRef = useRef(0);
  const welcomeHandledRef = useRef(false);
  const composingRef = useRef(false);
  const autosavePendingRef = useRef(false);
  const [view, setView] = useState<EditorView | null>(null);
  const [text, setText] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [panel, setPanel] = useState<Panel>(null);
  const [filePrompt, setFilePrompt] = useState<FilePrompt>(null);
  const [closePrompt, setClosePrompt] = useState(false);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const busyRef = useRef(false);
  const [busy, setBusy] = useState(false);
  const [languageBusy, setLanguageBusy] = useState<string | null>(null);
  const modalOpen = Boolean(panel || filePrompt || closePrompt);
  const modalOpenRef = useRef(false);
  modalOpenRef.current = modalOpen;

  const applyView = useCallback((next: EditorView, restoreCaret: boolean) => {
    viewRef.current = next;
    setView(next);
    if (composingRef.current) return;
    textRef.current = next.text;
    cursorRef.current = next.cursorUtf16;
    lastSelectedRef.current = next.cursorUtf16;
    setText(next.text);
    setError(null);
    if (restoreCaret) requestAnimationFrame(() => {
      const control = textarea.current;
      if (!control) return;
      control.focus({ preventScroll: true });
      control.setSelectionRange(next.cursorUtf16, next.cursorUtf16);
    });
  }, []);

  const run = useCallback((command: () => Promise<EditorView>, restoreCaret = false) => {
    const request = ++requestRef.current;
    return queue.current.run(command).then(next => {
      if (alive.current && request === requestRef.current) applyView(next, restoreCaret);
      return next;
    }).catch(reason => {
      if (alive.current) setError(message(reason));
      return null;
    });
  }, [applyView]);

  useEffect(() => {
    alive.current = true;
    if (isNativeDesktop) void run(native.snapshot).then(first => {
      if (first?.config.showWelcome && !welcomeHandledRef.current) {
        welcomeHandledRef.current = true;
        setPanel('welcome');
      }
    });
    return () => { alive.current = false; };
  }, [run]);

  const hasUnsavedChanges = useCallback(() => Boolean(
    viewRef.current?.documents.some(document => document.dirty) || textRef.current !== viewRef.current?.text || pendingKeysRef.current,
  ), []);

  const performAction = useCallback((name: ActionName) => {
    void run(() => native.action(name), name === actions.input || name === actions.reverseInput || name === actions.script);
  }, [run]);

  const changeLanguage = async (code: string, operation: 'select' | 'install' | 'remove') => {
    if (languageBusy) return;
    setLanguageBusy(code);
    await run(() => native.language(code, operation));
    setLanguageBusy(null);
  };

  const browse = useCallback((path: string, kind: 'open' | 'save', closeAfterSave = false) => {
    setFilePrompt(current => current?.kind === kind ? { ...current, path, browsing: true } : current);
    void queue.current.run(() => native.browse(path)).then(location => {
      if (!alive.current) return;
      setFilePrompt(current => current?.kind === kind && current.path === path ? {
        ...current, path: location.input, candidates: location.candidates, browsing: false, closeAfterSave,
      } : current);
    }).catch(reason => {
      if (alive.current) {
        setError(message(reason));
        setFilePrompt(current => current?.kind === kind ? { ...current, browsing: false } : current);
      }
    });
  }, []);

  const openFilePrompt = useCallback((kind: 'open' | 'save', closeAfterSave = false) => {
    setPanel(null);
    const path = kind === 'save' ? (viewRef.current?.path ?? '~/untitled.md') : (viewRef.current?.path ?? '~/');
    setFilePrompt({ kind, path, closeAfterSave, candidates: [], browsing: true });
    browse(path, kind, closeAfterSave);
  }, [browse]);

  const showClosePrompt = useCallback(() => {
    if (busyRef.current) return;
    if (hasUnsavedChanges()) {
      setPanel(null);
      setFilePrompt(null);
      setClosePrompt(true);
    } else {
      void destroyAppWindow().catch(reason => setError(message(reason)));
    }
  }, [hasUnsavedChanges]);
  useCloseProtection(showClosePrompt);

  const requestClose = useCallback(() => {
    void requestAppClose().catch(reason => setError(message(reason)));
  }, []);

  useEffect(() => {
    const seconds = view?.config.autosaveSecs ?? 0;
    if (seconds <= 0) return;
    const timer = window.setInterval(() => {
      if (
        autosavePendingRef.current || busyRef.current || composingRef.current || modalOpenRef.current ||
        !viewRef.current?.path || !viewRef.current.dirty
      ) return;
      autosavePendingRef.current = true;
      void run(() => native.save(null)).finally(() => { autosavePendingRef.current = false; });
    }, seconds * 1000);
    return () => window.clearInterval(timer);
  }, [hasUnsavedChanges, run, view?.config.autosaveSecs]);

  const requestSave = useCallback(() => {
    if (!viewRef.current?.path) openFilePrompt('save');
    else void run(() => native.save(null));
  }, [openFilePrompt, run]);

  // Flush composition before queuing navigation; block edits until the new buffer arrives.
  const navigateDocument = useCallback(async (command: () => Promise<EditorView>) => {
    if (busyRef.current) return;
    textarea.current?.blur();
    busyRef.current = true;
    setBusy(true);
    try { await run(command, true); }
    finally { busyRef.current = false; setBusy(false); }
  }, [run]);

  const newDocument = useCallback(() => {
    void navigateDocument(native.newDocument);
  }, [navigateDocument]);

  // The save flow pauses for each unnamed draft and resumes after its Save As dialog.
  const saveBeforeClose = async () => {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusy(true);
    try {
      let current = await run(native.snapshot);
      while (current) {
        const dirty = current.documents.find(document => document.dirty);
        if (!dirty) {
          await destroyAppWindow().catch(reason => setError(message(reason)));
          return;
        }
        current = await run(() => native.switchDocument(dirty.id), true);
        if (!current) return;
        if (!current.path) {
          setClosePrompt(false);
          openFilePrompt('save', true);
          return;
        }
        current = await run(() => native.save(null));
      }
    } finally { busyRef.current = false; setBusy(false); }
  };

  const handleFileSubmit = async (event: FormEvent) => {
    event.preventDefault();
    if (!filePrompt?.path.trim() || busyRef.current) return;
    const prompt = filePrompt;
    const path = prompt.path.trim();
    busyRef.current = true;
    setBusy(true);
    const result = await run(
      () => prompt.kind === 'open' ? native.open(path) : native.save(path),
      prompt.kind === 'open',
    );
    busyRef.current = false;
    setBusy(false);
    if (result) {
      setFilePrompt(null);
      if (prompt.closeAfterSave) await saveBeforeClose();
    }
  };

  const syncText = useCallback((nextText: string, cursor: number) => {
    textRef.current = nextText;
    cursorRef.current = cursor;
    lastSelectedRef.current = cursor;
    setText(nextText);
    void run(() => native.sync(nextText, cursor));
  }, [run]);

  const updateOptimisticText = (nextText: string, cursor: number) => {
    if (busyRef.current) return;
    textRef.current = nextText;
    cursorRef.current = cursor;
    lastSelectedRef.current = cursor;
    setText(nextText);
    if (!composingRef.current) syncText(nextText, cursor);
  };

  const handleSelection = () => {
    const control = textarea.current;
    if (!control || composingRef.current || busyRef.current) return;
    const cursor = control.selectionEnd;
    cursorRef.current = cursor;
    if (cursor === lastSelectedRef.current) return;
    lastSelectedRef.current = cursor;
    void run(() => native.select(cursor));
  };

  const handleEditorKey = (event: ReactKeyboardEvent<HTMLTextAreaElement>) => {
    if (
      busyRef.current || modalOpenRef.current || !view || view.inputMode === 'os' || event.nativeEvent.isComposing ||
      event.nativeEvent.keyCode === 229 || (event.key.length === 1 && !/^[\x20-\x7e]$/.test(event.key))
    ) return;
    if (event.metaKey || event.ctrlKey || /^F\d+$/.test(event.key)) return;
    const named = new Set(['Backspace', 'Delete', 'Enter', 'Tab', 'Escape', 'ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End']);
    if (event.key.length !== 1 && !named.has(event.key)) return;
    event.preventDefault();
    const control = event.currentTarget;
    const start = control.selectionStart;
    const end = control.selectionEnd;
    if (start !== end) {
      const navigation = ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'Escape'];
      if (navigation.includes(event.key)) {
        const cursor = event.key === 'ArrowLeft' ? start : end;
        lastSelectedRef.current = cursor;
        if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
          void run(() => native.select(cursor), true);
          return;
        }
        void run(() => native.select(cursor));
      } else {
        const withoutSelection = `${textRef.current.slice(0, start)}${textRef.current.slice(end)}`;
        syncText(withoutSelection, start);
        if (event.key === 'Backspace' || event.key === 'Delete') return;
      }
    }
    pendingKeysRef.current += 1;
    void run(() => native.key(event.key, false, event.altKey, event.shiftKey), true)
      .finally(() => { pendingKeysRef.current = Math.max(0, pendingKeysRef.current - 1); });
  };

  useEffect(() => {
    const handleShortcut = (event: KeyboardEvent) => {
      if (busyRef.current || event.defaultPrevented || event.isComposing || event.keyCode === 229 || composingRef.current) return;
      if (modalOpenRef.current) {
        if (event.key === 'Escape') {
          event.preventDefault(); setFilePrompt(null); setPanel(null); setClosePrompt(false);
        }
        return;
      }
      const modifier = event.metaKey || event.ctrlKey;
      if (event.altKey && event.key.toLowerCase() === 'p') { event.preventDefault(); performAction(actions.page); return; }
      if (event.altKey && event.key.toLowerCase() === 'l') { event.preventDefault(); performAction(actions.spacing); return; }
      if (modifier && event.key.toLowerCase() === 'n') { event.preventDefault(); newDocument(); return; }
      if (modifier && event.key.toLowerCase() === 'b') { event.preventDefault(); setSidebarOpen(open => !open); return; }
      if (modifier && event.key.toLowerCase() === 'o') { event.preventDefault(); openFilePrompt('open'); return; }
      if (modifier && event.key.toLowerCase() === 's') {
        event.preventDefault();
        if (event.shiftKey) openFilePrompt('save');
        else requestSave();
        return;
      }
      if (modifier && event.key.toLowerCase() === 'q') { event.preventDefault(); requestClose(); return; }
      if (modifier && event.key.toLowerCase() === 'k' && viewRef.current?.inputMode === 'ja') { event.preventDefault(); performAction(actions.script); return; }
      const keyActions: Record<string, ActionName> = {
        F2: event.shiftKey ? actions.reverseInput : actions.input,
        F3: actions.focus, F4: actions.big, F5: event.shiftKey ? actions.spacing : actions.page,
        F6: actions.theme, F7: actions.smaller, F8: actions.larger, F11: actions.soundProfile,
      };
      if (event.key === 'F1') { event.preventDefault(); setPanel('help'); }
      else if (event.key === 'F9') { event.preventDefault(); setPanel('language'); }
      else if (event.key === 'F10') { event.preventDefault(); setPanel('sound'); }
      else if (event.key === 'F12') { event.preventDefault(); openFilePrompt('save'); }
      else if (keyActions[event.key]) { event.preventDefault(); performAction(keyActions[event.key]); }
    };
    window.addEventListener('keydown', handleShortcut);
    return () => window.removeEventListener('keydown', handleShortcut);
  }, [filePrompt, newDocument, openFilePrompt, panel, performAction, requestClose, requestSave]);

  if (!isNativeDesktop) {
    return <main className="native-required"><div className="native-required__mark" aria-hidden="true">T</div><h1>{copy.en.native}</h1><p>{copy.en.nativeHint}</p></main>;
  }

  if (!view) {
    return <main className="loading-screen" role="status"><span className="loading-screen__cursor" aria-hidden="true" />{error ?? copy.en.loading}</main>;
  }

  const locale = view.config.language === 'ko' || view.config.language === 'ja' ? view.config.language : 'en';
  const t = copy[locale];
  const mode = view.inputMode === 'ko' ? 'IME:KO' : view.inputMode === 'ja' ? `IME:JA${view.candidate ? '変' : view.japaneseKatakana ? 'ア' : 'あ'}` : 'IME:OS';
  const pageStyle = {
    '--termleaf-fg': view.colors.fg, '--termleaf-bg': view.colors.bg,
    '--termleaf-dim': view.colors.dim, '--termleaf-accent': view.colors.accent,
    '--termleaf-pixel': view.colors.pixel, '--termleaf-pixel-off': view.colors.pixelOff,
    '--font-level': view.config.fontSize, '--line-level': view.config.lineSpacing,
  } as CSSProperties;
  const commandKey = navigator.platform.toLowerCase().includes('mac') ? '⌘' : 'Ctrl+';

  const shortcuts = [
    ['F1', t.help, () => setPanel('help')],
    ['F5', t.page, () => performAction(actions.page)], ['⇧F5', t.spacing, () => performAction(actions.spacing)],
    ['F10', t.soundMenu, () => setPanel('sound')],
    [`${commandKey}O`, t.open, () => openFilePrompt('open')], [`${commandKey}S`, t.save, requestSave], [`${commandKey}Q`, t.quit, requestClose],
    ['F3', t.focus, () => performAction(actions.focus)],
    ['F6', t.theme, () => performAction(actions.theme)], ['F7', `${t.size}−`, () => performAction(actions.smaller)],
    ['F8', `${t.size}+`, () => performAction(actions.larger)],
    ['F2', t.input, () => performAction(actions.input)], ['⇧F2', '←', () => performAction(actions.reverseInput)],
    ['F4', t.big, () => performAction(actions.big)], ['F9', t.language, () => setPanel('language')],
    ['F11', t.keys, () => performAction(actions.soundProfile)], ['F12', t.saveAs, () => openFilePrompt('save')],
  ] as const;

  return (
    <div className={`desktop-shell${view.config.focusMode ? ' desktop-shell--focus' : sidebarOpen ? '' : ' desktop-shell--collapsed'}`} style={pageStyle}>
      {!view.config.focusMode && <aside className="document-sidebar" aria-label={t.documents} inert={modalOpen || busy}>
        <div className="sidebar-heading">
          {sidebarOpen && <strong>TERMLEAF</strong>}
          <button type="button" className="sidebar-toggle" aria-label={sidebarOpen ? t.collapseSidebar : t.expandSidebar} title={`${sidebarOpen ? t.collapseSidebar : t.expandSidebar} (${commandKey}B)`} aria-expanded={sidebarOpen} aria-controls="open-documents" onClick={() => setSidebarOpen(open => !open)}>{sidebarOpen ? '‹' : '›'}</button>
        </div>
        <button type="button" className="new-writing" aria-label={t.newWriting} title={`${t.newWriting} (${commandKey}N)`} onClick={newDocument}><span aria-hidden="true">+</span>{sidebarOpen && t.newWriting}</button>
        <div id="open-documents" hidden={!sidebarOpen} className="open-documents">
          <div className="sidebar-section-title"><span>{t.documents}</span><span>{view.documents.length}</span></div>
          <ul className="document-list">{view.documents.map(document => {
            const active = document.id === view.activeDocumentId;
            const dirty = document.dirty || (active && text !== view.text);
            const label = document.label || `${t.untitled} ${document.id}`;
            return <li key={document.id}><button type="button" aria-current={active ? 'page' : undefined} title={document.path ?? label} onClick={() => { if (!active) void navigateDocument(() => native.switchDocument(document.id)); }}>
              <span className="document-list__name">{label}</span>
              {dirty && <span className="document-list__dirty" aria-label={t.unsaved}>●</span>}
            </button></li>;
          })}</ul>
          <button type="button" className="sidebar-open" onClick={() => openFilePrompt('open')}>{t.fileOpen}<kbd>{commandKey}O</kbd></button>
        </div>
      </aside>}
    <main className={`terminal-app${view.config.focusMode ? ' terminal-app--focus' : ''}${view.config.pageWidth ? ' terminal-app--page' : ''}`}>

      {view.config.bigFont && <div className="big-type" aria-hidden="true">{view.glyphs.map((glyph, index) => <BitmapGlyph glyph={glyph} key={index} />)}</div>}
      <div className="document-wrap" inert={modalOpen || busy}>
        <label className="visually-hidden" htmlFor="native-editor">Document</label>
        <textarea
          ref={textarea}
          id="native-editor"
          className="native-editor"
          value={text}
          readOnly={busy}
          onChange={event => updateOptimisticText(event.currentTarget.value, event.currentTarget.selectionEnd)}
          onSelect={handleSelection}
          onKeyDown={handleEditorKey}
          onCompositionStart={() => {
            composingRef.current = true;
            requestRef.current += 1;
          }}
          onCompositionEnd={event => {
            composingRef.current = false;
            syncText(event.currentTarget.value, event.currentTarget.selectionEnd);
          }}
          spellCheck={false}
          autoCapitalize="off"
          autoCorrect="off"
          aria-describedby={error ? 'editor-error' : undefined}
        />
      </div>

      {!view.config.focusMode && <footer className="terminal-chrome" inert={modalOpen || busy}>
        <div className="status-row" role="status" aria-live="polite">
          <span>{mode}</span><i />
          <span>{view.config.sound ? `${t.sound}:${view.config.soundProfile}` : t.muted}</span><i />
          <span>{view.dirty ? '*' : ''}{fileName(view.path, t.untitled)}</span><i />
          <span>{view.words} {t.words} {view.characters} {t.chars}</span><i />
          <span>{t.size.toLowerCase()} {view.config.fontSize}/5</span><i />
          <span>{t.line} {view.config.lineSpacing}/3</span><i />
          <span>{t.page}:{view.config.pageWidth ? t.on : t.off}</span><i />
          <span>{t.compose} {Array.from(view.composing).length ? '1/3' : '0/3'}</span>
          {view.candidate && <><i /><span className="candidate">{t.candidate}: {view.candidate}</span></>}
        </div>
        <nav className="shortcut-row" aria-label={t.helpTitle}>{shortcuts.map(([key, label, action]) => <button type="button" key={key} onClick={action}><kbd>{key}</kbd> {label}</button>)}</nav>
      </footer>}

      {error && <div id="editor-error" className="error-toast" role="alert"><span>{error}</span><button type="button" onClick={() => setError(null)}>×</button></div>}

      {filePrompt && <Overlay title={filePrompt.kind === 'open' ? t.fileOpen : t.fileSave} onClose={() => { if (!busyRef.current) setFilePrompt(null); }}>
        <form className="file-form" inert={busy} onSubmit={event => void handleFileSubmit(event)}>
          <label htmlFor="file-path">{t.fileHint}</label>
          <div className="path-row"><input id="file-path" autoFocus value={filePrompt.path} onChange={event => setFilePrompt({ ...filePrompt, path: event.currentTarget.value })} spellCheck={false} /><button type="button" onClick={() => browse(filePrompt.path, filePrompt.kind, filePrompt.closeAfterSave)}>{filePrompt.browsing ? '…' : t.browse}</button></div>
          {filePrompt.candidates.length > 0 && <div className="file-candidates" role="group" aria-label={t.fileOpen}>{filePrompt.candidates.map(candidate => <button type="button" key={candidate.path} onClick={() => candidate.isDir ? browse(candidate.path, filePrompt.kind, filePrompt.closeAfterSave) : setFilePrompt({ ...filePrompt, path: candidate.path })}><span aria-hidden="true">{candidate.isDir ? '▸' : '·'}</span><span>{candidate.name}</span></button>)}</div>}
          <div className="dialog__actions"><button type="button" onClick={() => setFilePrompt(null)}>{t.cancel}</button><button className="primary" type="submit">{filePrompt.kind === 'open' ? t.confirmOpen : t.confirmSave}</button></div>
        </form>
      </Overlay>}

      {closePrompt && <Overlay title={t.unsavedTitle} onClose={() => { if (!busyRef.current) setClosePrompt(false); }}>
        <div className="decision" inert={busy}><p>{t.closeQuestion}</p><div className="dialog__actions"><button type="button" onClick={() => setClosePrompt(false)}>{t.cancel}</button><button type="button" onClick={() => void destroyAppWindow().catch(reason => setError(message(reason)))}>{t.discard}</button><button className="primary" type="button" onClick={() => void saveBeforeClose()}>{t.saveAll}</button></div></div>
      </Overlay>}

      {(panel === 'help' || panel === 'welcome') && <Overlay title={panel === 'welcome' ? t.welcomeTitle : t.helpTitle} onClose={() => setPanel(null)}>
        {panel === 'welcome' && <p className="welcome-copy">{t.welcomeBody}</p>}
        <div className="help-grid">
          <kbd>{commandKey}N / {commandKey}B</kbd><span>{t.newWriting} / {t.documents}</span>
          <kbd>F2 / ⇧F2</kbd><span>{t.input}</span><kbd>F3 / F4</kbd><span>{t.focus} / {t.big}</span>
          <kbd>F5 / ⇧F5</kbd><span>{t.page} / {t.spacing}</span><kbd>F6 / F7 / F8</kbd><span>{t.theme} / {t.size}</span>
          <kbd>F9 / F10 / F11</kbd><span>{t.language} / {t.soundMenu} / {t.keys}</span><kbd>{commandKey}O / {commandKey}S / ⇧{commandKey}S</kbd><span>{t.open} / {t.save} / {t.saveAs}</span>
        </div>
        <div className="welcome-choice"><label><input type="checkbox" checked={!view.config.showWelcome} onChange={() => performAction(actions.welcome)} /> {t.dontShow}</label></div>
        <div className="dialog__actions"><button className="primary" type="button" onClick={() => setPanel(null)}>{t.close}</button></div>
      </Overlay>}

      {panel === 'language' && <Overlay title={t.languageTitle} onClose={() => setPanel(null)}>
        <div className="setting-list" aria-busy={Boolean(languageBusy)}>{view.languages.map(language => <div className="setting-row" key={language.code}><div><strong>{language.name}</strong><small>{languageBusy === language.code ? '…' : language.installed ? (view.config.language === language.code ? t.current : language.liveInput ? 'Live input' : '') : ''}</small></div><div>{language.installed && <button type="button" disabled={Boolean(languageBusy) || view.config.language === language.code} onClick={() => void changeLanguage(language.code, 'select')}>{t.use}</button>}{language.code !== 'en' && <button type="button" disabled={Boolean(languageBusy)} onClick={() => void changeLanguage(language.code, language.installed ? 'remove' : 'install')}>{language.installed ? t.remove : t.install}</button>}</div></div>)}</div>
      </Overlay>}

      {panel === 'sound' && <Overlay title={t.soundTitle} onClose={() => setPanel(null)}>
        <div className="sound-controls">
          <button type="button" onClick={() => performAction(actions.sound)}>{t.sound}: {view.config.sound ? t.on : t.off}</button>
          <button type="button" onClick={() => performAction(actions.soundProfile)}>{t.keys}: {view.config.soundProfile}</button>
          <button type="button" onClick={() => performAction(actions.backspaceSound)}>Backspace: {view.config.backspaceSound ? t.on : t.off}</button>
          <button type="button" onClick={() => performAction(actions.returnSound)}>Return: {view.config.returnSound ? t.on : t.off}</button>
        </div>
      </Overlay>}
    </main>
    </div>
  );
}
