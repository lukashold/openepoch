const CATALOG = 'local-original/fishing/manifest.json';
const CATALOG_HASH = 'fe43b6384fe6548d56c25766f4a697b424f3d09a9c09c632dac32e6cb9ec60a7';
const BOAT = 108, SCHOOL = 112;
const FAMILY = 'FishingShip';

export async function loadFishingPresentation({enabled, definition, fetchFile = globalThis.fetch,
  bitmap = globalThis.createImageBitmap, base = new URL('./', import.meta.url), profile = '7'} = {}) {
  if (!enabled || definition !== BOAT) return null;
  async function read(url, sha256) {
    if (!/^local-original\/[\w./-]+$/.test(url) || url.split('/').includes('..') || !/^[a-f0-9]{64}$/.test(sha256))
      throw Error('Invalid fishing media reference');
    const response = await fetchFile(new URL(url, base));
    if (!response.ok) throw Error('Fishing media unavailable');
    const data = await response.arrayBuffer();
    const digest = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', data)),
      byte => byte.toString(16).padStart(2, '0')).join('');
    if (digest !== sha256) throw Error('Fishing media checksum differs');
    return data;
  }
  try {
    const catalog = JSON.parse(new TextDecoder().decode(await read(CATALOG, CATALOG_HASH)));
    if (catalog.format !== 'ee2-native-fishing-presentation-v1' || catalog.models.length !== 4) return null;
    const modelRows = catalog.models.map(row => ({...row, fishingAsset: true}));
    const boats = modelRows.filter(row => row.id === BOAT && row.assetKind === 0);
    if (boats.length !== 3 || modelRows.filter(row => row.id === SCHOOL && row.assetKind === 1).length !== 1) return null;
    const textureHashes = new Map(catalog.textures.map(row => [row.url, row.sha256]));
    const events = {}, assets = {};
    const intervals = boats.map(row => {
      const effectiveEvents = {}, effectiveEventSources = {};
      for (const action of ['select', 'order', 'attack']) {
        effectiveEvents[action] = row.voice[action].map(key => {
          const event = catalog.audio[key], name = 'native_fishing:' + key;
          if (!event?.url.startsWith('local-original/audio/fishing/')) throw Error('Invalid fishing voice');
          events[name] = {file: event.file, volume: event.volume};
          assets[event.file] = {sha256: event.sha256};
          return name;
        });
        effectiveEventSources[action] = row.sourceDeclaration;
      }
      return {epoch: row.epoch, until: row.until, sourceEpoch: row.epoch + 1, sourceVisual: row.sourceVisual,
        effectiveEvents, effectiveEventSources};
    });
    const icons = await Promise.all(boats.map(async row => {
      if (!bitmap) return null;
      const icon = row.icon[profile];
      try {
        const image = await bitmap(new Blob([await read(icon.url, icon.sha256)]));
        if (image.width === icon.width && image.height === icon.height) return image;
        image.close?.();
      } catch {/* Procedural icon remains usable. */}
      return null;
    }));
    const indexFor = (id, epoch) => id === BOAT ? boats.findIndex(row => row.epoch <= epoch && epoch < row.until) : -1;
    return Object.freeze({
      modelRows, textureHashes, sailManifest: catalog.sailManifest, wakeProfiles: catalog.wakeProfiles, audioModelRows: boats,
      audioManifest: {format: 'ee2-native-family-audio-v1', events, assets, families: {[FAMILY]: {role: 'voice', intervals}}},
      icon: (id, epoch) => icons[indexFor(id, epoch)] || null,
      name: (id, epoch, count) => boats[indexFor(id, epoch)]?.[count > 1 ? 'plural' : 'label'].text,
      description: (id, epoch) => boats[indexFor(id, epoch)]?.description.text,
    });
  } catch {
    return null;
  }
}
