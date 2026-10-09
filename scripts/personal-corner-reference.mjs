#!/usr/bin/env node
// Generate test masks from the explicitly pinned, unmodified Lisse core.
// Its MIT notice is retained in docs/personal/LICENSE-Lisse.
import { readFileSync, writeFileSync, mkdirSync, mkdtempSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { resolve, dirname, join } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath, pathToFileURL } from 'node:url';

const source = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const [referenceArg, outputArg] = process.argv.slice(2);
if (!referenceArg || !outputArg || process.argv.length !== 4) {
  throw new Error('Usage: node scripts/personal-corner-reference.mjs <Lisse-checkout> <output-directory>');
}
const reference = resolve(referenceArg), output = resolve(outputArg);
const pin = readFileSync(join(source, 'docs/personal/versions.env'), 'utf8').match(/^LISSE_REV=(.+)$/m)[1];
const actual = execFileSync('git', ['-C', reference, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (actual !== pin) throw new Error(`Lisse mismatch: expected ${pin}, got ${actual}`);
execFileSync('git', ['-C', reference, 'diff', '--quiet', 'HEAD']);
const compiled = mkdtempSync(join(tmpdir(), 'niri-lisse-reference-'));
// Running from tmp avoids Volta selecting the reference repository's uninstalled dev dependencies.
execFileSync(process.env.NIRI_TSC || 'tsc', ['--project', join(reference, 'packages/core/tsconfig.json'),
  '--outDir', compiled, '--declaration', 'false', '--declarationMap', 'false'], { cwd: tmpdir(), stdio: 'inherit' });
writeFileSync(join(compiled, 'package.json'), '{"type":"module"}\n');
const { generatePath } = await import(pathToFileURL(join(compiled, 'generate-path.js')));
const cases = [
  ['uniform-065', 1, [40, 40, 40, 40], .65],
  ['uniform-001', 1, [40, 40, 40, 40], .01],
  ['uniform-100', 1, [40, 40, 40, 40], 1],
  ['fractional-065', 1.75, [40, 40, 40, 40], .65],
  ['asymmetric-065', 1.75, [48, 20, 0, 32], .65],
];
mkdirSync(output, { recursive: true });
for (const [name, scale, radii, smoothing] of cases) {
  const corners = Object.fromEntries(['topLeft', 'topRight', 'bottomRight', 'bottomLeft']
    .map((corner, index) => [corner, { radius: radii[index], smoothing }]));
  const path = generatePath(240, 160, corners);
  const svg = join(output, `${name}.svg`);
  writeFileSync(svg, `<svg xmlns="http://www.w3.org/2000/svg" width="${240 * scale}" height="${160 * scale}" viewBox="0 0 240 160"><path d="${path}" fill="white"/></svg>\n`);
  execFileSync('rsvg-convert', ['-o', join(output, `${name}.png`), svg]);
}
// A thin inset border at fractional scale: separately rasterize the pinned
// outer/inner paths, then integrate physical pixels with an 8x box filter.
for (const [name, width, height, radius, offset] of [
  ['border-175-outer', 240, 160, 40, 0],
  ['border-175-inner', 235, 155, 37.5, 2.5],
]) {
  const path = generatePath(width, height, { radius, smoothing: .65 });
  const svg = join(output, `${name}.svg`);
  writeFileSync(svg, `<svg xmlns="http://www.w3.org/2000/svg" width="420" height="280" viewBox="0 0 240 160"><g transform="translate(${offset} ${offset})"><path d="${path}" fill="white"/></g></svg>\n`);
  execFileSync('rsvg-convert', ['-z', '8', '-o', join(compiled, `${name}.png`), svg]);
}
execFileSync('python3', ['-c', `
from pathlib import Path
from PIL import Image
import sys
root, output = Path(sys.argv[1]), Path(sys.argv[2])
a = Image.open(root/'border-175-outer.png').getchannel('A').resize((420,280), Image.Resampling.BOX)
b = Image.open(root/'border-175-inner.png').getchannel('A').resize((420,280), Image.Resampling.BOX)
mask = Image.new('L', a.size)
mask.putdata([round(x * (255-y) / 255) for x,y in zip(getattr(a, "get_flattened_data", a.getdata)(), getattr(b, "get_flattened_data", b.getdata)())])
Image.merge('RGBA', [mask]*4).save(output/'border-175.png')
`, compiled, output]);
writeFileSync(join(output, 'source.json'), JSON.stringify({ repository: 'https://github.com/JaceThings/Lisse',
  commit: pin, generator: 'packages/core/src/generate-path.ts (TypeScript compiled without modification)',
  rasterizer: 'rsvg-convert, Cairo antialiasing', width: 240, height: 160, cases,
  border: { radius: 40, smoothing: .65, width: 2.5, scale: 1.75, supersample: 8, filter: 'Pillow BOX', composition: 'outer coverage * (1 - inner coverage)' } }, null, 2) + '\n');
console.log(output);
