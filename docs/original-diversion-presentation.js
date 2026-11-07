// Optional original media. Runtime IDs come from the current game schema.
const MANIFEST = 'local-original/diversion/manifest.json';
const DIGEST = '8ae10bc5c7451d3ae746a23fe05470789e3970fd450203c58ec3a79f24127fda';
const names = ['DiversionUnit_Epoch11', 'DiversionUnit_Epoch12', 'DiversionUnit_Epoch13', 'DiversionUnit_Epoch14'];
const starts = [10, 11, 12, 13, 15];
const hash = value => /^[a-f0-9]{64}$/.test(value || '');
const localPath = path => typeof path === 'string' && /^local-original\/[\w./-]+$/.test(path) && !path.split('/').includes('..');

async function verified(fetchFile, base, path, digest) {
  if (!localPath(path) || !hash(digest)) throw Error('Unverified Diversion asset');
  const response = await fetchFile(new URL(path, base));
  if (!response.ok) throw Error('Diversion asset unavailable');
  const bytes = await response.arrayBuffer();
  const actual = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), x => x.toString(16).padStart(2, '0')).join('');
  if (actual !== digest) throw Error('Diversion asset hash differs');
  return bytes;
}

export async function loadDiversionPresentation({enabled, firstDefinition, fetchFile = globalThis.fetch,
  base = new URL('./', import.meta.url)} = {}) {
  if (!enabled || !Number.isSafeInteger(firstDefinition) || firstDefinition < 0) return null;
  const read = async (url, sha) => JSON.parse(new TextDecoder().decode(await verified(fetchFile, base, url, sha)));
  try {
    const catalog = await read(MANIFEST, DIGEST);
    if (catalog.format !== 'ee2-native-diversion-presentation-v1' || catalog.rows.length !== names.length) return null;
    const modelRows = catalog.rows.map((row, i) => {
      if (row.visual !== names[i] || row.epoch !== starts[i] + 1 || row.through !== starts[i + 1] || row.models.length !== 1) throw Error('Unexpected Diversion epoch');
      const [model] = row.models;
      if (model.sourceVisual !== names[i] || model.assetKind !== 0 || !localPath(model.url) || !hash(model.sha256) ||
          (model.lodFarUrl && (!localPath(model.lodFarUrl) || !hash(model.lodFarSha256)))) throw Error('Unexpected Diversion model');
      return {...model, id: firstDefinition + i, until: row.through, regionalFallback: true, diversionVisual: row.visual};
    });
    const teamManifest = await read(catalog.teamManifest, catalog.teamManifestSha256);
    const rowFor = (id, epoch) => modelRows.findIndex(row => row.id === id && row.epoch <= epoch && epoch < row.until);
    const portraits = await Promise.all(catalog.rows.map(async ({portrait}) => {
      try {
        const bytes = await verified(fetchFile, base, portrait.url, portrait.sha256);
        const image = await createImageBitmap(new Blob([bytes], {type: 'image/png'}));
        if (image.width === 308 && image.height === 240) return image;
        image.close();
      } catch {}
    }));
    const sound = event => {
      const source = Object.hasOwn(catalog.audio.events, event) && catalog.audio.events[event];
      if (!source || !localPath(source.url) || !hash(source.sha256)) return null;
      return {...source, href: new URL(source.url, base).href};
    };
    const audioManifest = {format: 'ee2-native-family-audio-v1', families: {}, events: {}, assets: {}};
    const audioModelRows = modelRows.map((model, index) => {
      const sourceFamily = 'NativeDiversion' + (index + 11);
      const source = catalog.rows[index];
      const effectiveEvents = {}, effectiveEventSources = {};
      for (const action of ['select', 'order', 'attack']) {
        effectiveEvents[action] = source.voices[action].map(event => {
          const clip = sound(event), name = 'native_diversion:' + event;
          if (!clip?.url.startsWith('local-original/audio/')) throw Error('Missing original Diversion voice');
          const file = clip.url.slice('local-original/audio/'.length);
          audioManifest.events[name] = {file, volume: clip.volume};
          audioManifest.assets[file] = {sha256: clip.sha256};
          return name;
        });
        effectiveEventSources[action] = source.nativeDefinition;
      }
      audioManifest.families[sourceFamily] = {role: 'voice', intervals: [{epoch: model.epoch, until: model.until,
        sourceVisual: model.sourceVisual, sourceEpoch: source.epoch, effectiveEvents, effectiveEventSources}]};
      return {...model, sourceFamily};
    });
    const loadCommandArt = async () => {
      const css = async asset => {
        const bytes = await verified(fetchFile, base, asset.url, asset.sha256);
        return `url("${URL.createObjectURL(new Blob([bytes], {type: 'image/png'}))}")`;
      };
      const states = ['idle', 'hover', 'pressed'];
      try {
        const [buttons, cursors] = await Promise.all([
          Promise.all([1, 2, 3].map(theme => Promise.all(states.map(state => css(catalog.command.profiles[7][theme][state]))))),
          Promise.all(['invalid', 'valid'].map(async key => {
            const cursor = catalog.command.cursors[key];
            return `${await css(cursor)} ${cursor.hotspot.join(' ')}, ${key === 'valid' ? 'crosshair' : 'not-allowed'}`;
          })),
        ]);
        return {button: (_action, theme, state) => buttons[theme - 1]?.[states.indexOf(state)],
          cursor: (_action, valid) => cursors[Number(valid)]};
      } catch {
        return null;
      }
    };
    return Object.freeze({modelRows, teamManifest, command: catalog.command, audioModelRows, audioManifest,
      loadCommandArt,
      icon: (id, epoch) => portraits[rowFor(id, epoch)],
      hasVoices: (id, epoch) => {
        const row = catalog.rows[rowFor(id, epoch)];
        return row ? row.voices.select.length !== 0 : null;
      },
      name: (id, epoch) => catalog.rows[rowFor(id, epoch)]?.label?.text,
      voices: (id, epoch, role) => (catalog.rows[rowFor(id, epoch)]?.voices[role] || []).map(sound).filter(Boolean),
      state: (id, epoch, state) => catalog.rows[rowFor(id, epoch)]?.states[state],
      sound, readAsset: asset => verified(fetchFile, base, asset.url, asset.sha256),
    });
  } catch {
    return null;
  }
}
