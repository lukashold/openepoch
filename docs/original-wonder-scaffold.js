const PREFIX = 'local-original/wonders/scaffold/';
const SHA = /^[a-f0-9]{64}$/;
const SCALE = 0.02828427031636238;
const QUARTER_TURN = Math.fround(Math.PI / 2);

// 0063FE4B through 00640262, neighbor order -X, +X, -Y, +Y.
const TOP_MODULE = [4, 3, 3, 0, 3, 2, 2, 1, 3, 2, 2, 1, 0, 1, 1, 0];
const TOP_TURN = [0, -1, 1, 0, 2, -1, 2, -1, 0, 0, 1, 1, 0, 0, -2, 0];

function asset(row, path) {
  return row?.url === PREFIX + path && SHA.test(row.sha256) &&
    Number.isSafeInteger(row.bytes) && row.bytes > 0;
}

function axis(values) {
  return Array.isArray(values) && values.length > 0 && values.length <= 32 &&
    values.every((value, i) => Number.isFinite(value) && (i === 0 || value > values[i - 1]));
}

function bindLayout(row, moduleHeight) {
  const {centersX: x, centersY: y, centersZ: z} = row;
  if (![x, y, z].every(axis) || !Number.isFinite(row.height) || row.height <= 0 ||
      z.length !== Math.max(1, Math.trunc(row.height / moduleHeight)) || !Array.isArray(row.cells) ||
      row.cells.length === 0 || row.cells.length > x.length * y.length * z.length) {
    throw new Error('Ungültiges Originalgerüst-Raster');
  }
  const width = x.length + 2, depth = y.length + 2, layerSize = width * depth;
  const occupied = new Uint8Array(layerSize * (z.length + 1));
  const cells = row.cells.map(cell => {
    if (!Array.isArray(cell) || cell.length !== 3 || !cell.every((n, i) =>
      Number.isInteger(n) && n >= 0 && n < [x.length, y.length, z.length][i])) {
      throw new Error('Ungültige Originalgerüst-Zelle');
    }
    const [ix, iy, iz] = cell, at = (ix + 1) + (iy + 1) * width + iz * layerSize;
    if (occupied[at]) throw new Error('Doppelte Originalgerüst-Zelle');
    occupied[at] = 1;
    return {at, ix, iy, iz};
  });
  const byLayers = new Map();
  return fraction => {
    if (!(fraction > 0 && fraction < 1)) return null;
    let layers = Math.ceil(row.height * fraction / moduleHeight);
    if (layers > z.length) layers--;
    if (byLayers.has(layers)) return byLayers.get(layers);
    const parts = [];
    for (const {at, ix, iy, iz} of cells) {
      if (iz >= layers) continue;
      const mask = occupied[at - 1] | occupied[at + 1] << 1 |
        occupied[at - width] << 2 | occupied[at + width] << 3;
      const covered = iz + 1 < layers && occupied[at + layerSize];
      parts.push({module: covered ? 0 : TOP_MODULE[mask], yaw: covered ? 0 : TOP_TURN[mask] * QUARTER_TURN,
        x: x[ix] * SCALE, y: -y[iy] * SCALE, altitude: z[iz] * SCALE});
    }
    const stage = {layers, parts};
    byLayers.set(layers, stage);
    return stage;
  };
}

export function wonderScaffoldBindings(manifest, modelRows, bodyManifestSha256) {
  if (manifest?.format !== 'ee2-native-wonder-scaffold-v1' || !SHA.test(bodyManifestSha256) ||
      manifest.bodyManifestSha256 !== bodyManifestSha256 || manifest.sourceScale !== SCALE ||
      manifest.nativeConstructionProfile !== 3 ||
      !(manifest.moduleHeight > 0 && Number.isFinite(manifest.moduleHeight)) ||
      !Array.isArray(manifest.templates) || manifest.templates.length !== 5 ||
      !Array.isArray(manifest.textures) || !manifest.textures.length ||
      !Array.isArray(manifest.rows) || !manifest.rows.length || manifest.rows.length > modelRows.length) {
    throw new Error('Unbekanntes Originalgerüst-Format');
  }
  for (const [index, row] of manifest.templates.entries()) {
    if (!asset(row, `models/scafford7_r${index}.json`) || !SHA.test(row.sourceSha256)) {
      throw new Error('Ungültiges Originalgerüst-Modell');
    }
  }
  const textures = new Map();
  for (const row of manifest.textures) {
    const key = row?.url?.slice(PREFIX.length);
    if (!/^textures\/\w+\.png$/.test(key) || !asset(row, key) || textures.has(row.url)) {
      throw new Error('Ungültige Originalgerüst-Textur');
    }
    textures.set(row.url, row.sha256);
  }
  const rows = new Map();
  for (const row of manifest.rows) {
    const body = modelRows.find(model => model.id === row.id);
    if (!body || rows.has(row.id) || body.wonderFamily !== row.family || body.source.sha256 !== row.sourceSha256) {
      throw new Error('Originalgerüst passt nicht zum Weltwunder');
    }
    rows.set(row.id, {sourceSha256: row.sourceSha256, stage: bindLayout(row, manifest.moduleHeight)});
  }
  return {templates: manifest.templates, textures, rows, sourceScale: SCALE};
}

export async function loadWonderScaffold(wonders, {enabled, fetchFile = globalThis.fetch, base = import.meta.url} = {}) {
  if (!enabled || !wonders?.enabled || !globalThis.crypto?.subtle) return null;
  try {
    const response = await fetchFile(new URL(PREFIX + 'manifest.json', base), {cache: 'no-store'});
    if (!response.ok) return null;
    return wonderScaffoldBindings(await response.json(), wonders.modelRows, wonders.bodyManifestSha256);
  } catch {
    return null;
  }
}

export function wonderScaffoldStage(binding, data, offset) {
  const flags = Math.round(data[offset + 7]);
  return (flags & 64) && !(flags & 24) ? binding.stage(data[offset + 6]) : null;
}

export function scaffoldInstanceKey(data, offset) {
  return `${data[offset]}:${data[offset + 1]}:${data[offset + 2]}:${data[offset + 3]}:` +
    `${data[offset + 5]}:${data[offset + 6]}:${Math.round(data[offset + 7]) & ~64}`;
}
