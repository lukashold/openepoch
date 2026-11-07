import {loadAnimationPayload} from './original-animation-payload.js?v=citizen9';

const CATALOG = 'local-original/artemis/manifest.json';
const CATALOG_HASH = '1361749a0172fa087f8f80bea1ff78bb47ca952a92c0d685ea5a47dde35d2635';
const DEFINITION = 43;
const CLIPS = new Set(['idle_variant1', 'idle_variant2', 'death']);

export async function loadArtemisPresentation({enabled, fetchFile = globalThis.fetch, bitmap = globalThis.createImageBitmap,
  base = new URL('./', import.meta.url)} = {}) {
  if (!enabled) return null;
  async function read(url, sha) {
    if (!/^local-original\/artemis\/[\w./-]+$/.test(url) || url.split('/').includes('..') || !/^[a-f0-9]{64}$/.test(sha)) throw Error('Invalid Artemis asset');
    const response = await fetchFile(new URL(url, base));
    if (!response.ok) throw Error('Artemis asset unavailable');
    const bytes = await response.arrayBuffer();
    const digest = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), value => value.toString(16).padStart(2, '0')).join('');
    if (digest !== sha) throw Error('Artemis asset hash differs');
    return bytes;
  }
  const json = async (url, sha) => JSON.parse(new TextDecoder().decode(await read(url, sha)));
  try {
    const data = await json(CATALOG, CATALOG_HASH);
    if (data.format !== 'ee2-artemis-presentation-v1' || data.definition !== DEFINITION || data.from !== 13 || data.until !== 15) return null;
    let image = null;
    for (const profile of ['7', '5']) {
      const row = data.icons[profile];
      try {
        image = await bitmap(new Blob([await read(row.url, row.sha256)]));
        if (image.width === row.width && image.height === row.height) break;
        image.close?.(); image = null;
      } catch { image = null; }
    }
    const eligible = (id, epoch) => id === DEFINITION && data.from <= epoch && epoch < data.until;
    return Object.freeze({
      icon: (id, epoch) => eligible(id, epoch) ? image : null,
      name: (id, epoch, count) => eligible(id, epoch) ? (count > 1 ? data.plural : data.name).text : undefined,
      description: (id, epoch) => eligible(id, epoch) ? data.description.text : undefined,
      cue: name => Object.hasOwn(data.audio, name) ? data.audio[name] : null,
      async attachAdditionalPoses(parent) {
        const overlay = await json(data.poseSupplement.url, data.poseSupplement.sha256);
        if (overlay.format !== 'ee2-artemis-extra-poses-v1' || overlay.source.sha256 !== data.baseGeometrySha256 ||
            overlay.source.sha256 !== parent.source?.sha256 || JSON.stringify(overlay.normalization) !== JSON.stringify(parent.normalization) ||
            overlay.parts.length !== parent.parts.length) throw Error('Artemis pose source differs');
        const parts = parent.parts.map((part, index) => {
          const authored = overlay.parts[index];
          if (part.name !== authored.name || part.sourceNode !== authored.sourceNode || part.positions.length !== authored.vertices * 3 ||
              part.indices.length !== authored.indices || Object.keys(authored.animations).some(name => !CLIPS.has(name))) throw Error('Artemis pose topology differs');
          return {...part, animations: authored.animations};
        });
        const packed = await loadAnimationPayload({...parent, parts, animationPayload: overlay.animationPayload}, {fetchImpl: fetchFile, base});
        return {...parent, parts: parent.parts.map((part, index) => ({...part, animations: {...part.animations, ...packed.parts[index].animations}}))};
      },
    });
  } catch { return null; }
}
