const ROOT = 'local-original/territory-borders/';
const MANIFEST = '477fd35ef121bd694ac279348b2ac94e130448780ed2c102c656e63d3b5e093d';
const ALPHA = '1308f65a5bcbe8d00296fa54f7431b82ea30984bbca72fe920bbcb82cb3d431f';
const CLIMATES = ['arid', 'temperate', 'tropical', 'tundra'];
const SIDES = [128, 64, 32, 16, 8, 4, 2, 1];
const OFFSETS = [0, 16384, 20480, 21504, 21760, 21824, 21840, 21844];
const LAYERS = 30;
const LAYER_BYTES = 21845;

async function verifiedBytes(name, hash, base, fetchFile) {
  const url = new URL(ROOT + name, base);
  url.searchParams.set('v', hash.slice(0, 16));
  const response = await fetchFile(url);
  if (!response.ok) throw Error('Territory border media unavailable');
  const bytes = new Uint8Array(await response.arrayBuffer());
  const actual = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)),
    byte => byte.toString(16).padStart(2, '0')).join('');
  if (actual !== hash) throw Error('Territory border media checksum differs');
  return bytes;
}

export async function loadOriginalTerritoryBorders({enabled, fetchFile = globalThis.fetch,
  base = new URL('./', import.meta.url)} = {}) {
  if (!enabled) return null;
  try {
    const manifest = JSON.parse(new TextDecoder().decode(await verifiedBytes('manifest.json', MANIFEST, base, fetchFile)));
    const {alpha, layers, climates} = manifest;
    if (manifest.format !== 'ee2-private-territory-border-mip-atlas-v1' || manifest.localOnly !== true ||
        alpha.url !== ROOT + 'mips-r8.bin' || alpha.sha256 !== ALPHA ||
        alpha.length !== LAYERS * LAYER_BYTES || alpha.layerBytes !== LAYER_BYTES ||
        alpha.sides.join() !== SIDES.join() || alpha.offsets.join() !== OFFSETS.join() ||
        layers.length !== LAYERS || layers.some((layer, index) => layer.index !== index)) return null;
    const climateSlots = Object.fromEntries(CLIMATES.map(climate => {
      const rows = climates[climate];
      if (rows.length !== 15 || rows.some(states => states.length !== 3 ||
        states.some(({layer}) => !Number.isInteger(layer) || layer < 0 || layer >= LAYERS))) {
        throw Error('Territory border sprite table differs');
      }
      return [climate, rows.map(states => states.map(({layer}) => layer))];
    }));
    const pixels = await verifiedBytes('mips-r8.bin', ALPHA, base, fetchFile);
    if (pixels.byteLength !== alpha.length) return null;
    return {pixels, climateSlots};
  } catch {
    return null;
  }
}

export function uploadOriginalTerritoryBorders(gl, atlas, {minFilter, magFilter}) {
  if (atlas.pixels.byteLength !== LAYERS * LAYER_BYTES || minFilter === undefined || magFilter === undefined)
    throw Error('Invalid territory border texture upload');
  const target = gl.TEXTURE_2D_ARRAY;
  const previous = gl.getParameter(gl.TEXTURE_BINDING_2D_ARRAY);
  const alignment = gl.getParameter(gl.UNPACK_ALIGNMENT);
  const texture = gl.createTexture();
  if (!texture) throw Error('Cannot allocate territory border atlas');
  try {
    gl.bindTexture(target, texture);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texStorage3D(target, SIDES.length, gl.R8, SIDES[0], SIDES[0], LAYERS);
    for (const [level, side] of SIDES.entries()) {
      const area = side * side;
      const data = new Uint8Array(area * LAYERS);
      for (let layer = 0; layer < LAYERS; layer++) {
        const offset = layer * LAYER_BYTES + OFFSETS[level];
        data.set(atlas.pixels.subarray(offset, offset + area), layer * area);
      }
      gl.texSubImage3D(target, level, 0, 0, 0, side, side, LAYERS, gl.RED, gl.UNSIGNED_BYTE, data);
    }
    gl.texParameteri(target, gl.TEXTURE_MIN_FILTER, minFilter);
    gl.texParameteri(target, gl.TEXTURE_MAG_FILTER, magFilter);
    gl.texParameteri(target, gl.TEXTURE_WRAP_S, gl.MIRRORED_REPEAT);
    gl.texParameteri(target, gl.TEXTURE_WRAP_T, gl.MIRRORED_REPEAT);
    return texture;
  } catch (error) {
    gl.deleteTexture(texture);
    throw error;
  } finally {
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, alignment);
    gl.bindTexture(target, previous);
  }
}
