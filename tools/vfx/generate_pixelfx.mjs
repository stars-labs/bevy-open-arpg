#!/usr/bin/env node
// Export the five selected PixelFX effects with a shared dark-fantasy grade.
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync, unlinkSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const revision = '044e12a45dc9cc312cec50ee606dfc98afe5adc2';
const upstream = join(root, 'target/pixelfx-upstream');
if (!existsSync(upstream)) {
  execFileSync('git', ['clone', 'https://github.com/patch-gg/pixelfx.git', upstream], { stdio: 'inherit' });
  execFileSync('git', ['-C', upstream, 'checkout', '--detach', revision], { stdio: 'inherit' });
}
const actualRevision = execFileSync('git', ['-C', upstream, 'rev-parse', 'HEAD'], { encoding: 'utf8' }).trim();
if (actualRevision !== revision) throw new Error(`PixelFX source must be ${revision}, found ${actualRevision}`);
const { groundFrostNova } = await import('./frost_nova.mjs');
const core = await import(pathToFileURL(join(upstream, 'src/core/index.js')));
const { loadEffect, renderFrames, spriteSheet, contactSheet, save } = await import(pathToFileURL(join(upstream, 'tools/lib/fxlib.mjs')));
const output = join(root, 'assets/vfx/pixelfx');
const previews = join(root, 'target/validation/pixelfx/source');
mkdirSync(output, { recursive: true });
mkdirSync(previews, { recursive: true });

const effects = [
  { kind: 'Slash', name: 'slash', id: 'melee/sword-slash', duration: 0.32, start: 0, end: 0.65,
    palette: ['#171615', '#39342b', '#70634b', '#b49b6b', '#ead8ad'] },
  { kind: 'HeavyImpact', name: 'heavy_impact', id: 'impacts/blunt-hit', duration: 0.55, start: 0, end: 0.85,
    palette: ['#171414', '#3b2b24', '#78513a', '#b78a53', '#ead5aa'] },
  { kind: 'Fireball', name: 'fireball', id: 'fire/fireball', duration: 0.8, start: 0, end: 1.17,
    palette: ['#1b1211', '#4b2018', '#983b21', '#d17a36', '#f1d39a'] },
  { kind: 'FrostNova', name: 'frost_nova', id: 'ice/frost-nova', duration: 1.25, start: 0, end: 1.25,
    palette: ['#111b25', '#243946', '#466875', '#85a7ae', '#d5e4df'] },
  { kind: 'Summon', name: 'summon', id: 'spawn/summon-rise', duration: 1.1, start: 0.78, end: 3.8,
    palette: ['#111b19', '#28463e', '#4d8473', '#a0c0a2', '#e2e7cb'] },
];
function rgb(hex) {
  return [1, 3, 5].map(offset => parseInt(hex.slice(offset, offset + 2), 16));
}
function grade(pixels, palette) {
  const colors = palette.map(rgb);
  for (let offset = 0; offset < pixels.length; offset += 4) {
    if (!pixels[offset + 3]) continue;
    const light = Math.max(pixels[offset], pixels[offset + 1], pixels[offset + 2]) / 255;
    const position = Math.pow(light, 0.9) * (colors.length - 1);
    const low = Math.min(Math.floor(position), colors.length - 2);
    const fraction = position - low;
    for (let channel = 0; channel < 3; channel++) {
      pixels[offset + channel] = Math.round(colors[low][channel] * (1 - fraction) + colors[low + 1][channel] * fraction);
    }
  }
}
function writeGenerated(path, value) {
  const data = Buffer.isBuffer(value) ? value : Buffer.from(value);
  if (existsSync(path) && readFileSync(path).equals(data)) return;
  writeFileSync(path, data);
}
const manifest = [];
for (const effect of effects) {
  let temporaryModule;
  let fx;
  try {
    if (effect.kind === 'FrostNova') {
      const mod = groundFrostNova(core);
      fx = { id: effect.id, mod, meta: mod.meta };
    } else if (effect.kind === 'Summon') {
      const source = join(upstream, 'src/fx/spawn/summon-rise.js');
      const original = readFileSync(source, 'utf8');
      const bodyGuard = 'if (r > 0) {';
      if (original.split(bodyGuard).length !== 2) throw new Error('Summon body adapter needs review');
      temporaryModule = join(dirname(source), 'arpg-summon-rise.js');
      writeFileSync(temporaryModule, original.replace(bodyGuard, 'if (false) {'));
      fx = await loadEffect('spawn/arpg-summon-rise');
      fx.id = effect.id;
    } else {
      fx = await loadEffect(effect.id);
    }
    const rendered = renderFrames(fx, { actor: false, bg: 'none', seed: 9, options: { palette: effect.palette } });
    const first = Math.ceil(effect.start * rendered.fps);
    const last = Math.min(rendered.frames.length - 1, Math.floor(effect.end * rendered.fps));
    rendered.frames = rendered.frames.slice(first, last + 1);
    if (effect.kind === 'Slash') {
      // Start with a visible edge and hold the bright arc across low frame rates.
      const sourceFrames = rendered.frames;
      const timing = [1, 2, 3, 3, 4, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 19];
      rendered.frames = timing.map(index => ({
        ...sourceFrames[index], clear: sourceFrames[index].clear.slice(),
      }));
    }
    rendered.n = rendered.frames.length;
    rendered.fps = rendered.n / effect.duration;
    for (const frame of rendered.frames) {
      grade(frame.clear, effect.palette);
      // Review sheets use the same exported colors over a dark neutral background.
      frame.color = frame.clear;
    }
    const sheet = spriteSheet(fx, rendered, effect.name);
    const seconds = effect.duration / rendered.n;
    for (const frame of sheet.json.frames) frame.duration = seconds * 1000;
    sheet.json.meta.pixelfx.sourceRevision = revision;
    sheet.json.meta.pixelfx.adaptation = effect.kind === 'Summon' ? 'Ground runes, dust and rubble only; creature artwork omitted.' : effect.kind === 'FrostNova' ? 'Radial ground-plane adaptation using PixelFX ice crystals and expanding bands.' : 'Dark-fantasy palette and release-aligned timing.';
    delete sheet.json.meta.pixelfx.act;
    writeGenerated(join(output, `${effect.name}.png`), sheet.png);
    writeGenerated(join(output, `${effect.name}.json`), JSON.stringify(sheet.json, null, 2) + '\n');
    save(join(previews, `${effect.name}.png`), contactSheet(fx, rendered, { max: 12, zoom: 2, title: effect.name }));
    const meta = sheet.json.meta.pixelfx;
    const fields = [
      `kind: ${effect.kind}`,
      `size: (${rendered.w}.0, ${rendered.h}.0)`,
      `columns: ${meta.columns}`,
      `rows: ${Math.ceil(rendered.n / meta.columns)}`,
      `pivot: (${fx.meta.origin.map(value => value.toFixed(1)).join(', ')})`,
      `durations: [${Array(rendered.n).fill(seconds.toFixed(9)).join(', ')}]`,
      `looping: ${fx.meta.loop}`,
    ];
    manifest.push(`(${fields.join(', ')})`);
    console.log(`${effect.name}: ${rendered.n} frames, ${effect.duration}s, ${meta.columns} columns`);
  } finally {
    if (temporaryModule) unlinkSync(temporaryModule);
  }
}
writeGenerated(join(output, 'manifest.ron'), `// Generated by tools/vfx/generate_pixelfx.mjs.\n(effects: [\n${manifest.map(item => '    ' + item).join(',\n')}\n])\n`);
writeGenerated(join(output, 'LICENSE'), readFileSync(join(upstream, 'LICENSE')));
