import {prepareAuthoredWater} from './authored-water.js';

const sourceArchiveSha256 = '8f0f6028cda68dcaa395fffda9973f83a7f24ea22f2a12361cafa1fc392bcec4';
const inputSha256 = 'bad5aaa92c694ace5c467cb7c460c87c2ede4706d95a958c96368e006ad28b3e';
const inputBytes = 2971296;
const width = 100, height = 300;
const corners = (width + 1) * (height + 1);
const start = 84, paletteLength = 33;
const hex = bytes => Array.from(bytes, value => value.toString(16).padStart(2, '0')).join('');

/** Read forward source corners and original material movement classes from W8I1. */
export async function readDeathValley2WaterInput(bytes, {subtle = globalThis.crypto?.subtle} = {}) {
  if (!(bytes instanceof Uint8Array) || bytes.length !== inputBytes || !subtle) {
    throw new TypeError('Tal des Todes 2 benötigt den vollständigen W8I1-Kartendatensatz.');
  }
  const source = new Uint8Array(bytes);
  const digest = hex(new Uint8Array(await subtle.digest('SHA-256', source)));
  if (digest !== inputSha256) throw new Error('Prüfsumme der Originalkarte stimmt nicht.');
  const view = new DataView(source.buffer);
  if (view.getUint32(0, true) !== 0x31493857 || view.getUint32(4, true) !== 2 ||
      view.getUint32(8, true) !== width || view.getUint32(12, true) !== height ||
      view.getUint32(20, true) !== paletteLength || hex(source.subarray(52, start)) !== sourceArchiveSha256) {
    throw new RangeError('Ungültige Quelldaten für das Wasser von Tal des Todes 2.');
  }
  const elevations = Float32Array.from({length: corners}, (_, i) => view.getFloat32(start + 4 * i, true));
  const materials = Uint16Array.from({length: corners}, (_, i) => view.getUint16(start + 4 * corners + 2 * i, true));
  const paletteStart = start + 10 * corners;
  const movePalette = source.slice(paletteStart, paletteStart + paletteLength);
  return {width, height, elevations, materials, movePalette, sourceArchiveSha256, inputSha256};
}

export async function prepareDeathValley2Water(bytes, options = {}) {
  const input = await readDeathValley2WaterInput(bytes, options);
  return prepareAuthoredWater(input, options);
}
