const PREFIX = 'local-original/wonders/foundation/';
const TEXTURE_URL = 'local-original/wonders/scaffold/textures/scaffold1.png';
const TEXTURE_SHA = 'c0c0500edbfdfd807a793d1b7e3cea76708513f7734d5b22ea1fadc3c728df95';
const SHA = /^[a-f0-9]{64}$/;
const SCALE = 0.02828427031636238;
const AZIMUTH = [0, Math.fround(Math.PI), Math.fround(Math.PI / 2), Math.fround(3 * Math.PI / 2)];

function validAsset(row, path) {
  return row?.url === PREFIX + path && SHA.test(row.sha256) &&
    Number.isSafeInteger(row.bytes) && row.bytes > 0;
}

export function wonderFenceBindings(manifest, modelRows, bodySha256) {
  if (manifest?.format !== 'ee2-native-wonder-foundation-v1' || !SHA.test(bodySha256) ||
      manifest.bodyManifestSha256 !== bodySha256 || manifest.sourceScale !== SCALE ||
      manifest.nativeConstructionProfile !== 3 || !validAsset(manifest.foundation, 'models/foundation.json') ||
      !SHA.test(manifest.foundation.sourceSha256) || !Array.isArray(manifest.textures) || manifest.textures.length !== 1 ||
      !Array.isArray(manifest.rows) || manifest.rows.length !== modelRows.length) {
    throw new Error('Ungültiger Katalog für originale Baustelleneinfassungen');
  }
  const texture = manifest.textures[0];
  if (texture?.url !== TEXTURE_URL || texture.sha256 !== TEXTURE_SHA || !Number.isSafeInteger(texture.bytes) || texture.bytes <= 0) {
    throw new Error('Ungültige Baustellentextur');
  }
  const textures = new Map([[texture.url, texture.sha256]]);
  const rows = new Map();
  for (const row of manifest.rows) {
    const body = modelRows.find(model => model.id === row.id);
    if (!body || rows.has(row.id) || body.wonderFamily !== row.family || body.source.sha256 !== row.sourceSha256 ||
        !Array.isArray(row.size) || row.size.length !== 2 || !row.size.every(n => Number.isInteger(n) && n > 0 && n <= 32) ||
        !Number.isFinite(row.minZ)) throw new Error('Baustelleneinfassung passt nicht zum Weltwunder');
    const [width, depth] = row.size;
    if (!Array.isArray(row.segments) || row.segments.length !== 2 * (width + depth)) {
      throw new Error('Ungültiger Umfang einer Baustelleneinfassung');
    }
    const segments = row.segments.map((segment, index) => {
      const side = index < width ? 0 : index < 2 * width ? 1 : index < 2 * width + depth ? 2 : 3;
      if (!Array.isArray(segment) || segment.length !== 3 || !segment.every(Number.isFinite) ||
          segment[2] !== AZIMUTH[side]) throw new Error('Ungültiges Teil einer Baustelleneinfassung');
      return {x: segment[0] * SCALE, y: -segment[1] * SCALE, yaw: segment[2]};
    });
    rows.set(row.id, {sourceSha256: row.sourceSha256, altitude: row.minZ * SCALE, segments});
  }
  return {foundation: manifest.foundation, textures, rows, sourceScale: SCALE};
}

export async function loadWonderFence(wonders, {enabled, fetchFile = globalThis.fetch, base = import.meta.url} = {}) {
  if (!enabled || !wonders?.enabled || !globalThis.crypto?.subtle) return null;
  try {
    const response = await fetchFile(new URL(PREFIX + 'manifest.json', base), {cache: 'no-store'});
    if (!response.ok) return null;
    return wonderFenceBindings(await response.json(), wonders.modelRows, wonders.bodyManifestSha256);
  } catch {
    return null;
  }
}

export function wonderFenceAtSite(data, offset) {
  const flags = Math.round(data[offset + 7]);
  // Ordinary do_build makes the first admitted work tick positive. This
  // projection also reconstructs the site state from existing save bytes.
  return !!(flags & 64) && !(flags & 24) && data[offset + 6] === 0;
}
