const CATALOG = 'local-original/granary/manifest.json';
const CATALOG_HASH = '1a0631e190cd7bee6d64e87ec6c5d1b65c633b6e88585208b48f11fbf939339b';
const GRANARY = 109;
const FAMILY = 'Granary';
const STAGES = [[1, 5, 'Bld_windmill3'], [5, 10, 'Bld_windmill2'], [10, 14, 'Bld_granary'], [14, 15, 'Bld_granary15']];
const HASH = /^[a-f0-9]{64}$/;
const LOOP = 'fx_monawindmill';
const SELECT_EARLY = 'fx_monawindmill_sel1';
const SELECT_LATE = 'fx_monawindmill_sel2';

function checkedMedia(url, hash) {
  return typeof url === 'string' && /^local-original\/[\w./-]+$/.test(url)
    && !url.split('/').includes('..') && HASH.test(hash);
}

export async function loadGranaryPresentation({enabled, definition, fetchFile = globalThis.fetch,
  bitmap = globalThis.createImageBitmap, base = new URL('./', import.meta.url), profile = '7'} = {}) {
  if (!enabled || definition !== GRANARY || !['5', '7'].includes(profile)) return null;
  async function read(url, hash) {
    if (!checkedMedia(url, hash)) throw Error('Invalid Granary media reference');
    const source = new URL(url, base);
    source.searchParams.set('v', hash.slice(0, 16));
    const response = await fetchFile(source);
    if (!response.ok) throw Error('Granary media unavailable');
    const data = await response.arrayBuffer();
    const actual = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', data)),
      byte => byte.toString(16).padStart(2, '0')).join('');
    if (actual !== hash) throw Error('Granary media checksum differs');
    return data;
  }
  try {
    const catalog = JSON.parse(new TextDecoder().decode(await read(CATALOG, CATALOG_HASH)));
    if (catalog.format !== 'ee2-native-granary-presentation-v1' || catalog.definition !== GRANARY ||
        catalog.renderId !== 365 || catalog.models?.length !== STAGES.length) return null;
    const modelRows = catalog.models.map((row, index) => {
      const [epoch, until, body] = STAGES[index];
      const model = `cache/nifcache/${body}.nifcache`;
      if (row.id !== GRANARY || row.assetKind !== 0 || row.epoch !== epoch || row.until !== until ||
          row.sourceFamily !== FAMILY || row.sourceVisual !== `Granary_${epoch + 1}` ||
          row.source?.entry !== model || row.source.archive !== 'Graphics_160.zip' ||
          row.nativeScaleBinding?.sourceSha256 !== row.source.sha256 || !row.nativeScaleBinding.verified ||
          row.nativeScaleBinding.scaleType !== 'kScaleBuilding' ||
          row.nativeScaleBinding.sourceScale !== Math.fround(.04) || !checkedMedia(row.url, row.sha256) ||
          row.select !== (epoch === 14 ? SELECT_LATE : SELECT_EARLY) || row.idleLoop !== (epoch < 10 ? LOOP : null))
        throw Error('Granary source interval differs');
      return {...row, granaryAsset: true};
    });
    const textureHashes = new Map(catalog.textures.map(row => {
      if (!checkedMedia(row.url, row.sha256)) throw Error('Invalid Granary texture');
      return [row.url, row.sha256];
    }));
    if (textureHashes.size !== catalog.textures.length) throw Error('Duplicate Granary material');
    const {seasonManifest} = catalog;
    for (const sets of Object.values(seasonManifest.sets)) {
      for (const {urls} of Object.values(sets)) {
        if (![1, 12].includes(urls.length) || urls.some(url => !textureHashes.has(url)))
          throw Error('Invalid Granary season texture');
      }
    }
    for (const row of modelRows)
      if (seasonManifest.models[row.url]?.source?.sha256 !== row.source.sha256) throw Error('Granary seasons lack a source body');
    const audio = {};
    for (const key of [SELECT_EARLY, SELECT_LATE, LOOP]) {
      const row = catalog.audio[key];
      if (!checkedMedia(row?.url, row?.sha256) || row.url !== `local-original/audio/${row.file}` ||
          !row.file.startsWith('granary/') || row.volume !== (key === LOOP ? .4 : 1) ||
          row.loop !== (key === LOOP) || row.maximumInstances !== (key === LOOP ? 4 : 0))
        throw Error('Invalid Granary source audio');
      audio[key] = row;
    }
    const events = Object.fromEntries([SELECT_EARLY, SELECT_LATE].map(key => [
      key, {file: audio[key].file, volume: audio[key].volume},
    ]));
    const assets = Object.fromEntries(Object.values(audio).map(row => [row.file, {sha256: row.sha256}]));
    const intervals = modelRows.map(row => ({epoch: row.epoch, until: row.until, sourceEpoch: row.epoch + 1,
      sourceVisual: row.sourceVisual, effectiveEvents: {select: [row.select], order: [], attack: []},
      effectiveEventSources: {select: `db/TechTree/upgrade_unittype5.csv:GranaryUpgradeEpoch${row.epoch + 1}`}}));
    const icons = await Promise.all(modelRows.map(async row => {
      if (!bitmap) return null;
      const icon = row.icon[profile];
      try {
        const image = await bitmap(new Blob([await read(icon.url, icon.sha256)], {type: 'image/png'}),
          {premultiplyAlpha: 'none', colorSpaceConversion: 'none'});
        if (image.width === icon.width && image.height === icon.height) return image;
        image.close?.();
      } catch {/* Existing procedural icon remains available. */}
      return null;
    }));
    const indexFor = (id, epoch) => id === GRANARY ? modelRows.findIndex(row => row.epoch <= epoch && epoch < row.until) : -1;
    return Object.freeze({
      modelRows, textureHashes, seasonManifest, audioModelRows: modelRows, idleLoop: audio[LOOP],
      audioManifest: {format: 'ee2-native-family-audio-v1', events, assets, families: {[FAMILY]: {role: 'selection-effect', intervals}}},
      icon: (id, epoch) => icons[indexFor(id, epoch)] ?? null,
      name: (id, epoch, count) => modelRows[indexFor(id, epoch)]?.[count > 1 ? 'plural' : 'label'].text,
      description: (id, epoch) => modelRows[indexFor(id, epoch)]?.description.text,
    });
  } catch {
    return null;
  }
}

export function createGranaryLoopPlayback(event, {ready, buffer, load, start, stop}) {
  const voices = new Map();
  let session = null;
  function reset() {
    for (const source of voices.values()) stop(source);
    voices.clear();
    session = null;
  }
  function update(world, tracks, audible) {
    if (!ready() || !world || !tracks?.length) { reset(); return; }
    if (session !== world) reset();
    session = world;
    const wanted = new Map(tracks.filter(row => audible(row.volume)).map(row => [row.handle, row]));
    for (const [handle, source] of voices) {
      if (wanted.has(handle)) continue;
      voices.delete(handle);
      stop(source);
    }
    const clip = buffer(event.file);
    if (!clip) { if (wanted.size) load(event.file); return; }
    for (const row of [...wanted.values()].sort((a, b) => b.volume - a.volume || a.handle - b.handle)) {
      const gain = event.volume * row.volume;
      const previous = voices.get(row.handle);
      if (previous) { previous.updateSpatialMix(gain, row.pan); continue; }
      if (voices.size >= event.maximumInstances) continue;
      const source = start(clip, gain, row.pan, 'granary-idle', true);
      if (!source) continue;
      voices.set(row.handle, source);
      const ended = source.onended;
      source.onended = () => {
        ended?.();
        if (voices.get(row.handle) === source) voices.delete(row.handle);
      };
    }
  }
  return Object.freeze({update, reset});
}
