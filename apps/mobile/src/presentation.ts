import glyphData from './glyphs.json' with { type: 'json' };

// Same palettes as src/ui/themes.rs. The font data is generated from the Rust assets.
export const themes = {
  paper: { fg: '#282622', bg: '#f4f0e8', dim: '#969187', accent: '#b44632', pixel: '#282622', pixelOff: '#e0dbd1' },
  night: { fg: '#f2f2f2', bg: '#000000', dim: '#5c5c5c', accent: '#ffffff', pixel: '#f2f2f2', pixelOff: '#1c1c1c' },
  xt: { fg: '#4cff70', bg: '#000501', dim: '#0c4a1b', accent: '#b2ffbc', pixel: '#4cff70', pixelOff: '#021c08' },
  amber: { fg: '#ffb000', bg: '#140c00', dim: '#966400', accent: '#ffdc78', pixel: '#ffb000', pixelOff: '#372200' },
} as const;
export type ThemeName = keyof typeof themes;
export type Palette = { [K in keyof typeof themes.paper]: string };
export const themeNames = Object.keys(themes) as ThemeName[];

export interface Glyph { width: number; rows: readonly number[] }
const data: Readonly<Record<string, readonly number[]>> = glyphData;
const fallback: Glyph = { width: 8, rows: [255, 129, 129, 129, 129, 129, 129, 129, 129, 255] };
export function glyphFor(character: string): Glyph {
  const glyph = data[character.codePointAt(0) ?? -1];
  return glyph ? { width: glyph[0]!, rows: glyph.slice(1) } : fallback;
}

/** Like Editor::focus_text: current line, immediately before the UTF-16 caret. */
export function focusText(text: string, cursorUtf16: number, maxCharacters: number): string {
  if (maxCharacters <= 0) return '';
  let end = Math.max(0, Math.min(text.length, cursorUtf16));
  // Never split a surrogate pair if a native selection event is mid-character.
  if (end > 0 && /[\uD800-\uDBFF]/.test(text[end - 1]!) && /[\uDC00-\uDFFF]/.test(text[end] ?? '')) end--;
  return Array.from(text.slice(0, end).split('\n').at(-1) ?? '').slice(-maxCharacters).join('');
}

export function documentLabel(document: { title: string; body: string }, fallback = 'untitled'): string {
  return document.title.trim() || document.body.split('\n').find(line => line.trim())?.trim().slice(0, 40) || fallback;
}
