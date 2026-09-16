import { memo, useEffect, useRef, useState } from 'react';
import {
  AppState, Keyboard, KeyboardAvoidingView, Modal, Platform, Pressable,
  ScrollView, StyleSheet, Text, TextInput, View, useWindowDimensions,
} from 'react-native';
import { StatusBar } from 'expo-status-bar';
import { SafeAreaProvider, SafeAreaView } from 'react-native-safe-area-context';
import { useNotebook } from '@termleaf/notebook/react';
import type { Draft } from '@termleaf/notebook';
import fontLicense from './src/fontLicense.json';
import { mobileStorage } from './src/storage';
import { documentLabel, focusText, glyphFor, themes, themeNames, type Palette, type ThemeName } from './src/presentation';

const mono = Platform.OS === 'ios' ? 'Menlo' : 'monospace';

function Command({ label, onPress, colors, disabled = false, selected = false }: {
  label: string; onPress: () => void; colors: Palette; disabled?: boolean; selected?: boolean;
}) {
  return <Pressable accessibilityRole="button" accessibilityLabel={label}
    accessibilityState={{ disabled, selected }} disabled={disabled} onPress={onPress}
    style={({ pressed }) => [styles.command, { opacity: disabled ? 0.4 : pressed ? 0.6 : 1 }]}>
    <Text style={[styles.commandText, { color: selected ? colors.accent : colors.fg }]}>{label}</Text>
  </Pressable>;
}

const PixelGlyph = memo(function PixelGlyph({ character, cell, colors }: { character: string; cell: number; colors: Palette }) {
  const glyph = glyphFor(character);
  return <View style={{ width: glyph.width * cell, height: 10 * cell }}>
    {glyph.rows.flatMap((row, y) => Array.from({ length: glyph.width }, (_, x) =>
      <View key={`${y}:${x}`} style={{ position: 'absolute', left: x * cell, top: y * cell,
        width: Math.max(0.5, cell - 0.35), height: Math.max(0.5, cell - 0.35),
        backgroundColor: (row >> (glyph.width - 1 - x)) & 1 ? colors.pixel : colors.pixelOff }} />,
    ))}
  </View>;
});

function WritingSurface({ draft, colors, size, spacing, big, pageWidth, keyboardVisible, initialCursor, rememberCursor, onChange }: {
  draft: Draft; colors: Palette; size: number; spacing: number; big: boolean; pageWidth: boolean;
  keyboardVisible: boolean; initialCursor: number; rememberCursor: (cursor: number) => void; onChange: (body: string) => void;
}) {
  const { width, height } = useWindowDimensions();
  const [cursor, setCursor] = useState(initialCursor);
  // Only restore selection when mounting a document, allowing native IMEs to own subsequent selection.
  const [restoredSelection, setRestoredSelection] = useState<{ start: number; end: number } | undefined>({ start: initialCursor, end: initialCursor });
  useEffect(() => { setRestoredSelection(undefined); }, []);
  const preview = Array.from(focusText(draft.body, cursor, 12));
  const cell = Math.min(4 + size * 1.6, keyboardVisible ? 7 : 12);
  const available = Math.max(0, width - 40);
  let pixels = 0;
  const visible: string[] = [];
  for (let index = preview.length - 1; index >= 0; index--) {
    const character = preview[index]!;
    const next = glyphFor(character).width * cell + cell;
    if (pixels + next > available) break;
    pixels += next;
    visible.unshift(character);
  }
  return <View style={styles.writingSurface}>
    {big && <View accessible={false} accessibilityElementsHidden importantForAccessibility="no-hide-descendants"
      style={[styles.pixelZone, { height: Math.min(keyboardVisible ? 96 : 160, height * 0.23), gap: cell }]}>
      {visible.map((character, index) => <PixelGlyph key={index} character={character} cell={cell} colors={colors} />)}
    </View>}
    <TextInput
      accessibilityLabel="Document" value={draft.body} multiline scrollEnabled
      selection={restoredSelection} selectionColor={colors.accent}
      onChangeText={body => {
        const next = Math.max(0, Math.min(body.length, cursor + body.length - draft.body.length));
        setCursor(next); rememberCursor(next); onChange(body);
      }}
      onSelectionChange={event => { const next = event.nativeEvent.selection.end; setCursor(next); rememberCursor(next); }}
      autoCapitalize="none" autoCorrect={false} spellCheck={false} textAlignVertical="top"
      style={[styles.editor, { color: colors.fg, lineHeight: 16 * spacing }, pageWidth && styles.pageWidth]}
    />
  </View>;
}

export default function App() {
  const notebook = useNotebook(mobileStorage);
  const { width } = useWindowDimensions();
  const [theme, setTheme] = useState<ThemeName>('night');
  const [focus, setFocus] = useState(false);
  const [big, setBig] = useState(true);
  const [size, setSize] = useState(2);
  const [spacing, setSpacing] = useState(2);
  const [pageWidth, setPageWidth] = useState(false);
  const [keyboardVisible, setKeyboardVisible] = useState(false);
  const [panel, setPanel] = useState<'documents' | 'help' | 'rename' | 'license' | null>(null);
  const [name, setName] = useState('');
  const cursors = useRef(new Map<string, number>());
  const initialized = useRef(false);
  const colors = themes[theme];
  const draft = notebook.activeDocument;
  const label = draft ? documentLabel(draft) : 'untitled';
  const body = draft?.body ?? '';
  const characters = Array.from(body.replaceAll('\n', '')).length;
  const words = body.trim().split(/\s+/).filter(Boolean).length;
  const status = notebook.status === 'loading' ? 'opening…' : notebook.status === 'saving' ? 'saving…' : notebook.status === 'error' ? 'save failed' : 'saved locally';

  useEffect(() => {
    if (notebook.ready && !initialized.current) {
      initialized.current = true;
      if (notebook.documents.length === 0) notebook.createDocument();
    }
  }, [notebook.ready, notebook.documents.length, notebook.createDocument]);

  useEffect(() => {
    const subscription = AppState.addEventListener('change', state => {
      if (state === 'inactive' || state === 'background') void notebook.save();
    });
    return () => subscription.remove();
  }, [notebook.save]);

  useEffect(() => {
    const show = Keyboard.addListener('keyboardDidShow', () => setKeyboardVisible(true));
    const hide = Keyboard.addListener('keyboardDidHide', () => setKeyboardVisible(false));
    return () => { show.remove(); hide.remove(); };
  }, []);

  const openPanel = (next: 'documents' | 'help' | 'rename') => {
    Keyboard.dismiss();
    if (next === 'rename') setName(draft?.title ?? '');
    setPanel(next);
  };
  const newWriting = () => { notebook.createDocument(); setPanel(null); };
  const cycleTheme = () => setTheme(themeNames[(themeNames.indexOf(theme) + 1) % themeNames.length]!);
  const command = (text: string, action: () => void, disabled = false) =>
    <Command key={text} label={text} onPress={action} disabled={disabled} colors={colors} />;

  return <SafeAreaProvider>
    <StatusBar style={theme === 'paper' ? 'dark' : 'light'} />
    <SafeAreaView style={[styles.safeArea, { backgroundColor: colors.bg }]}
      accessibilityElementsHidden={panel !== null} importantForAccessibility={panel ? 'no-hide-descendants' : 'auto'}>
      <KeyboardAvoidingView style={styles.app} behavior={Platform.OS === 'ios' ? 'padding' : undefined}>
        <View style={styles.topBar}>
          {command('Menu', () => openPanel('documents'))}
          {focus ? command('Exit focus', () => setFocus(false)) :
            <Pressable accessibilityRole="button" accessibilityLabel="Rename document" disabled={!draft} onPress={() => openPanel('rename')} style={styles.documentName}>
              <Text numberOfLines={1} style={[styles.fileName, { color: colors.fg }]}>{label}</Text>
            </Pressable>}
        </View>
        {!notebook.ready ? <View style={styles.loading}>
          <Text accessibilityRole={notebook.error ? 'alert' : undefined} style={[styles.message, { color: colors.fg }]}>{notebook.error ?? 'Opening Termleaf…'}</Text>
          {notebook.error && command('Retry', () => { void notebook.retry(); })}
        </View> : draft ? <WritingSurface key={draft.id} draft={draft} colors={colors} size={size} spacing={spacing} big={big}
          pageWidth={pageWidth} keyboardVisible={keyboardVisible} initialCursor={cursors.current.get(draft.id) ?? draft.body.length}
          rememberCursor={cursor => cursors.current.set(draft.id, cursor)} onChange={next => notebook.updateDocument({ body: next })} /> : <View style={styles.writingSurface} />}
        {!focus && <View style={styles.chrome}>
          <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerStyle={styles.statusRow}>
            <Text style={[styles.status, { color: colors.fg }]}>IME:OS | {status} | {label} | {words} words {characters} chars | size {size}/5 | line {spacing}/3 | page:{pageWidth ? 'on' : 'off'}</Text>
          </ScrollView>
          <ScrollView horizontal keyboardShouldPersistTaps="always" showsHorizontalScrollIndicator={false} contentContainerStyle={styles.commands}>
            {command('New', newWriting, !notebook.ready)}
            {command('Save', () => { void notebook.save(); }, !draft || notebook.status === 'saving')}
            {command('Focus', () => setFocus(true))}
            {command('Theme', cycleTheme)}
            {command('Size−', () => setSize(value => Math.max(1, value - 1)), size === 1)}
            {command('Size+', () => setSize(value => Math.min(5, value + 1)), size === 5)}
            {command('Spacing', () => setSpacing(value => value % 3 + 1))}
            {command('Big', () => setBig(value => !value))}
            {command('Page', () => setPageWidth(value => !value))}
            {command('Help', () => openPanel('help'))}
          </ScrollView>
        </View>}
        {notebook.ready && notebook.error && <View style={[styles.error, { borderColor: colors.accent }]}>
          <Text accessibilityRole="alert" style={[styles.message, { color: colors.fg }]}>{notebook.error}</Text>
          {command('Retry', () => { void notebook.retry(); })}
        </View>}
      </KeyboardAvoidingView>
    </SafeAreaView>
    <Modal visible={panel !== null} transparent animationType="fade" onRequestClose={() => setPanel(null)}>
      <View style={styles.overlay}>
        <Pressable accessibilityRole="button" accessibilityLabel="Close menu" onPress={() => setPanel(null)} style={StyleSheet.absoluteFill} />
        <SafeAreaView accessibilityViewIsModal style={[styles.drawer, { width: Math.min(360, width * 0.86), backgroundColor: colors.bg, borderRightColor: colors.dim }]} edges={['top', 'bottom', 'left']}>
          <View style={styles.drawerHeader}>
            <Text accessibilityRole="header" style={[styles.brand, { color: colors.fg }]}>TERMLEAF</Text>
            {command('Close', () => setPanel(null))}
          </View>
          {panel === 'documents' && <>
            <Pressable accessibilityRole="button" accessibilityLabel="New writing" disabled={!notebook.ready} onPress={newWriting}
              style={[styles.newWriting, { borderColor: colors.dim }]}>
              <Text style={[styles.commandText, { color: colors.fg }]}>+ New writing</Text>
            </Pressable>
            <Text accessibilityRole="header" style={[styles.sectionLabel, { color: colors.fg }]}>Open documents · {notebook.documents.length}</Text>
            <ScrollView style={styles.documentList} keyboardShouldPersistTaps="handled" contentContainerStyle={styles.documentListContent}>
              {notebook.documents.map(document => {
                const active = document.id === draft?.id;
                return <Pressable key={document.id} accessibilityRole="button" accessibilityLabel={`Open ${documentLabel(document)}`}
                  accessibilityState={{ selected: active }} onPress={() => { notebook.selectDocument(document.id); setPanel(null); }}
                  style={({ pressed }) => [styles.documentRow, { borderColor: active ? colors.accent : 'transparent', backgroundColor: active ? colors.pixelOff : 'transparent', opacity: pressed ? 0.6 : 1 }]}>
                  <Text numberOfLines={1} style={[styles.fileName, { color: active ? colors.accent : colors.fg }]}>{documentLabel(document)}</Text>
                </Pressable>;
              })}
            </ScrollView>
            <View style={[styles.drawerFooter, { borderColor: colors.dim }]}>
              {command('Help', () => setPanel('help'))}
              <Text accessibilityLiveRegion="polite" style={[styles.status, { color: colors.fg }]}>{status}</Text>
            </View>
          </>}
          {panel === 'rename' && <View style={styles.panelBody}>
            <Text accessibilityRole="header" style={[styles.message, { color: colors.fg }]}>Document name</Text>
            <TextInput accessibilityLabel="Document name" autoFocus value={name} onChangeText={setName} autoCorrect={false}
              selectionColor={colors.accent} style={[styles.nameInput, { color: colors.fg, borderColor: colors.dim }]} />
            {command('Rename', () => { notebook.updateDocument({ title: name }); setPanel(null); })}
          </View>}
          {panel === 'license' && <ScrollView contentContainerStyle={styles.panelBody}><Text selectable style={[styles.message, { color: colors.fg }]}>{fontLicense}</Text></ScrollView>}
          {panel === 'help' && <ScrollView contentContainerStyle={styles.panelBody}>
            <Text accessibilityRole="header" style={[styles.message, { color: colors.accent }]}>Termleaf</Text>
            <Text style={[styles.message, { color: colors.fg }]}>Write with your keyboard. The large pixel letters follow the cursor, just as they do on desktop.</Text>
            <Text style={[styles.message, { color: colors.fg }]}>Menu: new writing and open documents. Tap the document name to rename it. Changes save on this device as you write.</Text>
            <Text style={[styles.message, { color: colors.fg }]}>Swipe the bottom commands to reach size, spacing, big text and page width. Focus hides the bottom controls.</Text>
            {command('Font license', () => setPanel('license'))}
            <Text style={[styles.message, { color: colors.fg }]}>Galmuri9 2.40.4 · © 2019–2025 Lee Minseo. SIL Open Font License 1.1.</Text>
          </ScrollView>}
        </SafeAreaView>
      </View>
    </Modal>
  </SafeAreaProvider>;
}

const styles = StyleSheet.create({
  safeArea: { flex: 1 },
  app: { flex: 1 },
  topBar: { flexDirection: 'row', alignItems: 'center', minHeight: 44, paddingHorizontal: 8 },
  command: { minHeight: 44, minWidth: 44, paddingHorizontal: 10, justifyContent: 'center', alignItems: 'center' },
  commandText: { fontFamily: mono, fontSize: 12, fontWeight: '700' },
  documentName: { flex: 1, minHeight: 44, justifyContent: 'center', paddingHorizontal: 12 },
  fileName: { fontFamily: mono, fontSize: 12 },
  writingSurface: { flex: 1, minHeight: 0 },
  pixelZone: { flexDirection: 'row', justifyContent: 'center', alignItems: 'center', overflow: 'hidden', paddingHorizontal: 16 },
  editor: { flex: 1, minHeight: 0, width: '100%', alignSelf: 'center', paddingHorizontal: 22, paddingTop: 8, paddingBottom: 16, fontFamily: mono, fontSize: 16, borderWidth: 0, outlineWidth: 0, outlineStyle: 'solid' },
  pageWidth: { maxWidth: 720 },
  chrome: { paddingHorizontal: 8 },
  statusRow: { paddingHorizontal: 10, paddingTop: 4, paddingBottom: 2 },
  status: { fontFamily: mono, fontSize: 10, lineHeight: 16 },
  commands: { alignItems: 'center' },
  loading: { flex: 1, justifyContent: 'center', alignItems: 'center', padding: 24 },
  message: { fontFamily: mono, fontSize: 13, lineHeight: 22 },
  error: { margin: 12, padding: 12, borderWidth: 1 },
  overlay: { flex: 1, backgroundColor: 'rgba(0,0,0,0.5)' },
  drawer: { flex: 1, borderRightWidth: 1 },
  drawerHeader: { flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between', paddingHorizontal: 16, paddingVertical: 8 },
  brand: { fontFamily: mono, fontSize: 11, letterSpacing: 2 },
  newWriting: { minHeight: 44, justifyContent: 'center', alignItems: 'center', marginHorizontal: 16, marginTop: 8, marginBottom: 24, borderWidth: 1, borderRadius: 3 },
  sectionLabel: { fontFamily: mono, fontSize: 11, paddingHorizontal: 24, marginBottom: 12 },
  documentList: { flex: 1 },
  documentListContent: { paddingHorizontal: 16, gap: 4, paddingBottom: 16 },
  documentRow: { minHeight: 44, justifyContent: 'center', padding: 10, borderWidth: 1, borderRadius: 3 },
  drawerFooter: { marginHorizontal: 16, borderTopWidth: 1, flexDirection: 'row', alignItems: 'center', justifyContent: 'space-between' },
  panelBody: { padding: 22, gap: 20 },
  nameInput: { minHeight: 44, padding: 10, borderWidth: 1, fontFamily: mono, fontSize: 14 },
});
