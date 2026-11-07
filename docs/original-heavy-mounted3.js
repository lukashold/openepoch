const CATALOG = 'local-original/heavy-mounted3/manifest.json';
const CATALOG_HASH = 'cf45cb82e94503bcd1066d93a6b14f8a07328f8cf06cdfd2da04876aa36c7f1c';
const CATALOG_DEFINITION = 106;
const VOICE = 'HeavyMounted3';

export async function loadHeavyMounted3Presentation({enabled, definition, fetchFile = globalThis.fetch, bitmap = globalThis.createImageBitmap,
  base = new URL('./', import.meta.url), profile = '7'} = {}) {
  if (!enabled || !Number.isSafeInteger(definition) || definition < 0) return null;
  async function read(url, hash) {
    if (!/^local-original\/[\w./-]+$/.test(url) || url.split('/').includes('..') || !/^[a-f0-9]{64}$/.test(hash)) throw Error('Invalid HeavyMounted3 media reference');
    const response = await fetchFile(new URL(url, base));
    if (!response.ok) throw Error('HeavyMounted3 media unavailable');
    const data = await response.arrayBuffer();
    const digest = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', data)), byte => byte.toString(16).padStart(2, '0')).join('');
    if (digest !== hash) throw Error('HeavyMounted3 media hash differs');
    return data;
  }
  const json = async (url, sha) => JSON.parse(new TextDecoder().decode(await read(url, sha)));
  try {
    const source = await json(CATALOG, CATALOG_HASH);
    if (source.format !== 'ee2-native-heavy-mounted3-presentation-v1' || source.def !== CATALOG_DEFINITION ||
        source.epoch !== 13 || source.until !== 15 || source.model.sourceVisual !== VOICE) return null;
    const teamManifest = await json(source.teamManifest, source.teamManifestSha256);
    const modelRows = [{...source.model, id: definition}], events = {}, assets = {}, effectiveEvents = {}, effectiveEventSources = {};
    for (const action of ['select', 'order', 'attack']) {
      effectiveEvents[action] = source.voices[action].map(key => {
        const event = source.audio[key], name = 'native_heavy_mounted3:' + key;
        if (!event?.url.startsWith('local-original/audio/')) throw Error('Missing HeavyMounted3 voice');
        const file = event.url.slice('local-original/audio/'.length);
        events[name] = {file, volume: event.volume};
        assets[file] = {sha256: event.sha256};
        return name;
      });
      effectiveEventSources[action] = source.sourceReferences.upgrade.source;
    }
    const audioManifest = {format: 'ee2-native-family-audio-v1', events, assets, families: {
      [VOICE]: {role: 'voice', intervals: [{epoch: source.epoch, until: source.until, sourceEpoch: source.sourceEpoch,
        sourceVisual: VOICE, effectiveEvents, effectiveEventSources}]},
    }};
    const row = source.icon[profile];
    let image = null;
    if (row && bitmap) {
      try {
        image = await bitmap(new Blob([await read(row.url, row.sha256)]));
        if (image.width !== row.width || image.height !== row.height) { image.close?.(); image = null; }
      } catch {/* Missing icon keeps the procedural control. */}
    }
    const eligible = (id, epoch) => id === definition && epoch >= source.epoch && epoch < source.until;
    return Object.freeze({modelRows, teamManifest, audioModelRows: modelRows, audioManifest,
      icon: (id, epoch) => eligible(id, epoch) ? image : null,
      name: (id, epoch, count) => eligible(id, epoch) ? (count > 1 ? source.plural : source.label).text : undefined,
      description: (id, epoch) => eligible(id, epoch) ? source.description.text : undefined,
      cue: name => Object.hasOwn(source.audio, name) ? source.audio[name] : null,
      state: name => Object.hasOwn(source.states, name) ? source.states[name] : null,
      projectile: source.projectile,
      readAsset: record => read(record.url, record.sha256),
    });
  } catch {
    return null;
  }
}
