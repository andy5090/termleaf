// Galmuri9 2.40.4, SIL OFL-1.1. Reuse the terminal's exact bitmap data.
import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

const root = new URL('../', import.meta.url);
const glyphs = {};
const decode = (bytes, offset) => [bytes[offset], ...Array.from({ length: 10 }, (_, row) => bytes.readUInt16BE(offset + 1 + row * 2))];
const core = readFileSync(new URL('assets/galmuri9-2.40.4-core.bin', root));
if (core.length !== 95 * 21) throw Error('Unexpected core glyph length');
for (let index = 0; index < 95; index++) glyphs[index + 32] = decode(core, index * 21);
for (const language of ['ko', 'ja']) {
  const bytes = readFileSync(new URL(`language-packs/${language}/glyphs.bin`, root));
  const count = bytes.readUInt32BE(5);
  if (bytes.length !== 9 + count * 25) throw Error(`Invalid ${language} glyph pack`);
  for (let index = 0; index < count; index++) {
    const offset = 9 + index * 25;
    glyphs[bytes.readUInt32BE(offset)] = decode(bytes, offset + 4);
  }
}
const output = `${JSON.stringify(glyphs)}\n`;
const target = fileURLToPath(new URL('apps/mobile/src/glyphs.json', root));
if (process.argv.includes('--check')) {
  if (readFileSync(target, 'utf8') !== output) throw Error('Mobile glyphs differ from terminal assets; run node scripts/generate-mobile-glyphs.mjs');
} else {
  writeFileSync(target, output);
}
console.log(`${Object.keys(glyphs).length} shared Galmuri glyphs ${process.argv.includes('--check') ? 'verified' : 'generated'}`);

const license = JSON.stringify(readFileSync(new URL('assets/OFL-1.1.txt', root), 'utf8')) + '\n';
const licenseTarget = new URL('apps/mobile/src/fontLicense.json', root);
if (process.argv.includes('--check')) {
  if (readFileSync(licenseTarget, 'utf8') !== license) throw Error('Mobile font license is outdated');
} else writeFileSync(licenseTarget, license);
