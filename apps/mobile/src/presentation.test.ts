import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { documentLabel, focusText, glyphFor, themes } from './presentation.ts';

test('mobile ASCII and Korean/Japanese glyphs exactly match terminal source data', () => {
  const core = readFileSync(new URL('../../../assets/galmuri9-2.40.4-core.bin', import.meta.url));
  for (let code = 32; code <= 126; code++) {
    const offset = (code - 32) * 21;
    assert.equal(glyphFor(String.fromCodePoint(code)).width, core[offset]);
    assert.deepEqual(glyphFor(String.fromCodePoint(code)).rows, Array.from({length:10}, (_,row)=>core.readUInt16BE(offset + 1 + row * 2)));
  }
  for (const language of ['ko','ja']) {
    const bytes = readFileSync(new URL(`../../../language-packs/${language}/glyphs.bin`, import.meta.url));
    for (let offset = 9; offset < bytes.length; offset += 25) {
      const glyph = glyphFor(String.fromCodePoint(bytes.readUInt32BE(offset)));
      assert.equal(glyph.width, bytes[offset + 4]);
      assert.deepEqual(glyph.rows, Array.from({length:10}, (_,row)=>bytes.readUInt16BE(offset + 5 + row * 2)));
    }
  }
});

test('focus follows the caret on its current line without splitting Unicode', () => {
  assert.equal(focusText('old\n한글🌿日本語', 9, 3), '글🌿日');
  assert.equal(focusText('abcdef', 3, 2), 'bc');
  assert.equal(focusText('a🌿b', 2, 4), 'a');
  assert.equal(focusText('a\n', 2, 8), '');
  assert.equal(focusText('hello', 5, 0), '');
  assert.equal(focusText('ㅎ', 1, 6), 'ㅎ');
  assert.equal(focusText('하', 1, 6), '하');
  assert.equal(focusText('한', 1, 6), '한');
});

test('legacy titles remain labels and unnamed documents use their first nonblank line', () => {
  assert.equal(documentLabel({title:'Existing title',body:'Body'}), 'Existing title');
  assert.equal(documentLabel({title:'',body:'\n 첫 문장\nmore'}), '첫 문장');
  assert.equal(documentLabel({title:'',body:''}), 'untitled');
  assert.equal(glyphFor('🌿').width, 8);
  assert.equal(themes.night.bg, '#000000');
});
