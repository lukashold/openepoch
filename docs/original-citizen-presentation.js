import {citizenPresentationCatalog} from './original-citizen-models.js?v=citizen9';
import {isSourceCitizenContext} from './source-citizen-context.js?v=citizen9';

const iconPath = /^local-original\/citizens9\/icons\/[a-z0-9_.-]+\.png$/i;
const recordingPath = /^local-original\/citizens9\/audio\/[a-z0-9_.-]+\.mp3$/i;
const validHash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);

// All URLs remain inside the portable game's private media directory. The
// catalog resolves native M/F + epoch + regional source bindings; only playback
// round-robin and the existing build-to-move response are own UI adapters.
export async function loadCitizenPresentation({
  enabled = false,
  fetchFile = globalThis.fetch,
  bitmapFile = globalThis.createImageBitmap,
  manifestURL = new URL('./local-original/citizens9/presentation.json', import.meta.url),
  base = new URL('./', import.meta.url),
} = {}) {
  const images = new Map();
  const assets = new Map();
  const errors = [];
  let catalog = null;
  const api = {
    enabled: false,
    isSourceContext: isSourceCitizenContext,
    icon(id, epoch, context) {
      if (!isSourceCitizenContext(id, context)) return undefined;
      const row = catalog?.lookup(context.sourceIdentity, context.civilization, epoch);
      return images.get(row?.icon?.url);
    },
    resolveAudio(id, action, epoch, context) {
      if (!isSourceCitizenContext(id, context) || !catalog) return null;
      const channel = action === 'build' ? 'order' : action;
      if (!['select', 'order', 'attack'].includes(channel)) return null;
      const {sourceIdentity, civilization} = context;
      const row = catalog.lookup(sourceIdentity, civilization, epoch);
      if (!row) return null;
      const records = catalog.sounds(sourceIdentity, civilization, epoch, channel).map(record => ({
        ...record, file: new URL(record.recording, base).href,
      }));
      if (!records.length) return null;
      return {
        namespace: 'native-citizen', sourceFamily: row.sourceFamily, sourceIdentity, civilization,
        sourceEpoch: row.epoch, action: channel, records, group: 'voice', gain: 1,
        key: `citizen:${sourceIdentity}:${civilization}:${row.epoch}:${channel}`,
        legacyContextFallback: row.legacyContextFallback,
      };
    },
    usedFiles: () => [...assets.keys()],
    assetHash: file => assets.get(file),
    status: () => ({enabled: api.enabled, icons: images.size, recordings: assets.size, errors: [...errors]}),
  };
  if (!enabled) return api;
  try {
    const response = await fetchFile(manifestURL, {cache: 'no-cache'});
    if (!response.ok) throw Error('Citizen presentation manifest unavailable');
    const manifest = await response.json();
    for (const icon of Object.values(manifest.icons)) {
      if (!iconPath.test(icon.url) || !validHash(icon.pngSha256)) throw Error('Invalid Citizen icon binding');
    }
    for (const event of Object.values(manifest.events)) {
      if (!recordingPath.test(event.recording) || !validHash(event.sha256)
          || !Number.isFinite(event.volume) || event.volume < 0 || event.volume > 1) {
        throw Error('Invalid Citizen recording binding');
      }
      const url = new URL(event.recording, base).href;
      const previous = assets.get(url);
      if (previous && previous !== event.sha256) throw Error('Conflicting Citizen recording hash');
      assets.set(url, event.sha256);
    }
    for (const row of manifest.rows) {
      if (!Object.hasOwn(manifest.icons, row.icon)) throw Error('Unbound Citizen icon');
      for (const events of Object.values(row.soundEvents)) {
        if (!Array.isArray(events) || events.some(name => !Object.hasOwn(manifest.events, name))) {
          throw Error('Unbound Citizen sound event');
        }
      }
    }
    // Neutral slot 14 uses the western presentation in the own game.
    catalog = citizenPresentationCatalog(manifest, {neutralFallback: 'west'});
    await Promise.all(Object.values(manifest.icons).map(async icon => {
      try {
        const url = new URL(icon.url, base);
        url.searchParams.set('v', icon.pngSha256.slice(0, 16));
        const response = await fetchFile(url, {cache: 'force-cache'});
        if (!response.ok) throw Error('Citizen icon unavailable: ' + icon.url);
        const bitmap = await bitmapFile(await response.blob(), {premultiplyAlpha: 'none', colorSpaceConversion: 'none'});
        images.set(icon.url, bitmap);
      } catch (error) {
        errors.push(error.message);
      }
    }));
    api.enabled = true;
  } catch (error) {
    catalog = null;
    assets.clear();
    errors.push(error.message);
  }
  return api;
}
