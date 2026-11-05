import {modelSeasonFrames} from './original-model-seasons.js';

function checkedExtent(width, height, cells, ArrayType) {
 if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1 ||
     width > 5001 || height > 5001 || !(cells instanceof ArrayType) || cells.length !== width * height) {
  throw new RangeError('Invalid source terrain grid extent');
 }
}

/** Explicit adapter for the current six-class procedural map renderer. */
export function ownMapTerrainOrigin({types, width, height}, terrainManifest) {
 checkedExtent(width, height, types, Uint8Array);
 // These are the primary existing renderer roles. The procedural shader can
 // blend additional roles, so this is not a native DbTerrain ID conversion.
 const role = ['coral', 'beach', 'beach', 'grass', 'forest', 'rock'][types[0]];
 const terrainName = terrainManifest?.runtimeRoles?.[role]?.terrainType;
 if (typeof terrainName !== 'string' || !terrainName) throw new RangeError('Missing own terrain material role');
 return Object.freeze({kind: 'own-render-role', terrainName, ownClass: types[0], role});
}

function resolveOrigin(table, origin) {
 if (origin?.kind === 'own-render-role') {
  return {terrainName: origin.terrainName, ownClass: origin.ownClass, role: origin.role};
 }
 if (origin?.kind !== 'runtime-u16' && origin?.kind !== 'saved-u16') {
  throw new TypeError('Explicit native terrain origin context required');
 }
 checkedExtent(origin.width, origin.height, origin.materialIds, Uint16Array);
 // Every source model binder passes NiPoint3's initialized zero vector.
 // The native +0.5/binary64-bias/SAR16 conversion returns (0,0) exactly.
 const materialId = origin.materialIds[0];
 if (origin.kind === 'runtime-u16') return {materialId, terrainName: table.rows[materialId]?.name};
 if (!Array.isArray(origin.terrainNames)) throw new TypeError('Original map terrain names are required');
 return {materialId, terrainName: origin.terrainNames[materialId]};
}

export function createCityCenterTerrainBinding(manifest, loadTexture) {
 if (manifest?.format !== 'ee2-model-terrain-fallback-v1' || !manifest.modelBindings || !manifest.climates ||
     manifest.originCell?.[0] !== 0 || manifest.originCell?.[1] !== 0 || typeof loadTexture !== 'function') {
  throw new Error('Unsupported original model terrain fallback manifest');
 }
 let revision = 0;
 let active = null;

 function resolve(context) {
  const {climate, detailCode, origin} = context;
  const table = Object.hasOwn(manifest.climates, climate) ? manifest.climates[climate] : null;
  if (!table || !Number.isInteger(detailCode) || detailCode < -2147483648 || detailCode > 2147483647) {
   throw new RangeError('Explicit source climate and native terrain detail code required');
  }
  const source = resolveOrigin(table, origin);
  if (typeof source.terrainName !== 'string') throw new RangeError('Unknown original terrain material');
  const row = table.rows.findIndex(entry => entry.name.toLowerCase() === source.terrainName.toLowerCase());
  const detailSlot = detailCode === 0 ? '0' : detailCode === 1 ? '1' : 'other';
  const set = table.rows[row]?.detail[detailSlot];
  const urls = table.sets[set]?.urls;
  if (!Array.isArray(urls) || !urls.length) throw new RangeError('No original terrain seasonal set');
  return Object.freeze({kind: origin.kind, ...source, cell: Object.freeze([0, 0]), sourceRow: row,
                        climate, detailCode, set, urls: Object.freeze([...urls])});
 }

 async function bind(context) {
  if (context === null) {
   revision++;
   active = null;
   return null;
  }
  // Capture the cell and source set before awaiting textures. The selected set
  // is a material binding, independent of later actors, ticks and map writes.
  const selection = resolve(context);
  const requested = ++revision;
  const frames = active?.selection.climate === selection.climate && active.selection.set === selection.set
   ? active.frames : await Promise.all(selection.urls.map(loadTexture));
  if (requested !== revision) return false;
  active = {selection, frames};
  return selection;
 }

 function matches(modelUrl, model, part) {
  const record = manifest.modelBindings[modelUrl];
  const source = record?.parts?.[part.sourceNode];
  return !!source && record.sourceSha256 === model.source?.sha256 && source.name === part.name &&
    source.baseTextureSource === part.textureSource;
 }

 return Object.freeze({
  bind,
  matches,
  snapshot: () => active?.selection ?? null,
  frames(phase) {
   return active && Number.isFinite(phase) ? modelSeasonFrames(active.frames, phase, null) : null;
  },
 });
}
