import {createAmbientUpdate, createPresentationRandom} from './source-music-controller.js?v=regional11';
import {createMenuMusicUpdate} from './source-menu-music.js?v=regional12';

const regions = ['West', 'FarEast', 'MiddleEast', 'MesoAmerican'];
const civilizationRegions = [0, 0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3];
const epochRanges = [['Stone', 'Industrial'], ['Modern', 'Digital'], ['Genetic', 'Synthetic']];
const DEFAULT_MANIFEST = new URL('./local-original/music11/manifest.json', import.meta.url);

function validateManifest(data) {
  if (data?.format !== 'ee2-original-music-sets-v1' || data.regionSets?.length !== regions.length
      || data.epochSets?.length !== epochRanges.length || !data.assets) throw Error('Unsupported original soundtrack');
  for (const [name, asset] of Object.entries(data.assets)) {
    if (!/^[a-z0-9_]+\.mp3$/i.test(name) || asset.url !== 'local-original/music11/' + name
        || !/^[a-f0-9]{64}$/.test(asset.sha256)) throw Error('Invalid soundtrack file binding');
  }
  data.regionSets.forEach((row, index) => {
    if (row.region !== regions[index]) throw Error('Invalid soundtrack region');
  });
  data.epochSets.forEach((row, index) => {
    if (row.first !== epochRanges[index][0] || row.last !== epochRanges[index][1]) throw Error('Invalid soundtrack era');
  });
  for (const row of [...data.regionSets, ...data.epochSets]) {
    if (!Array.isArray(row.files) || row.files.some(name => !Object.hasOwn(data.assets, name))) throw Error('Unbound soundtrack file');
  }
  if (!Array.isArray(data.menu) || data.menu.length !== 2 || data.menu.some(name => !Object.hasOwn(data.assets, name))) {
    throw Error('Unbound menu soundtrack');
  }
  const {fracSilentPieces, timeSilenceLow, timeSilenceHigh, numLoopsLow, numLoopsHigh} = data.settings;
  if (!Number.isFinite(fracSilentPieces) || fracSilentPieces < 0 || fracSilentPieces > 1
      || !Number.isFinite(timeSilenceLow) || !Number.isFinite(timeSilenceHigh) || timeSilenceLow < 0 || timeSilenceLow > timeSilenceHigh
      || !Number.isInteger(numLoopsLow) || !Number.isInteger(numLoopsHigh) || numLoopsLow < 0
      || numLoopsLow > numLoopsHigh || numLoopsHigh > 65535) throw Error('Invalid soundtrack timing');
  return data;
}

// Original selection and loop accounting use an observed stream. The browser
// adapts that stream to decoded Web Audio buffers and pauses on user request.
export function createRegionalMusicPlayback(context, output, {
  fetchFile = globalThis.fetch,
  base = new URL('./', import.meta.url),
  manifestURL = DEFAULT_MANIFEST,
  random = createPresentationRandom(1),
  onStart = () => {},
  volume = .32,
} = {}) {
  let manifest = null, controller = null, menu = null, mode = null, status = 'loading', error = null;
  let active = null, fading = null, request = null, readyPiece = null;
  let audible = false, lastTime = null;
  const cache = new Map(), failed = new Set();
  const ready = Promise.resolve().then(async () => {
    const response = await fetchFile(manifestURL, {cache: 'no-cache'});
    if (!response.ok) throw Error('Original soundtrack unavailable');
    manifest = validateManifest(await response.json());
    controller = createAmbientUpdate(manifest.settings, random);
    menu = createMenuMusicUpdate(manifest.menu, random);
    status = 'ready';
    return true;
  }).catch(reason => { status = 'unavailable'; error = reason.message; return false; });

  function position(record) {
    const elapsed = record.source ? context.currentTime - record.started : 0;
    return (record.offset + Math.max(0, elapsed)) % record.buffer.duration;
  }

  function disconnect(record) {
    if (!record?.source) return;
    const source = record.source;
    source.onended = null;
    try { source.stop(); } catch { /* The browser may have ended this node. */ }
    source.disconnect();
    record.gain.disconnect();
    record.source = null;
  }

  function attach(record, fadeInSeconds = 0) {
    const source = context.createBufferSource();
    source.buffer = record.buffer;
    source.loop = true;
    const gain = context.createGain();
    const now = context.currentTime;
    gain.gain.setValueAtTime(fadeInSeconds ? 0 : volume, now);
    if (fadeInSeconds) gain.gain.linearRampToValueAtTime(volume, now + fadeInSeconds);
    source.connect(gain);
    gain.connect(output);
    record.source = source;
    record.gain = gain;
    record.started = now;
    record.rampUntil = now + fadeInSeconds;
    source.start(0, record.offset);
  }

  function fadeActive(seconds) {
    if (!active) return;
    disconnect(fading);
    fading = active;
    active = null; // 00513120 clears the active stream before the next update.
    const record = fading;
    if (!record.source) { fading = null; return; }
    const now = context.currentTime;
    const gain = record.gain.gain;
    if (gain.cancelAndHoldAtTime) gain.cancelAndHoldAtTime(now);
    else { gain.cancelScheduledValues(now); gain.setValueAtTime(gain.value, now); }
    gain.linearRampToValueAtTime(0, now + seconds);
    record.source.onended = () => {
      disconnect(record);
      if (fading === record) fading = null;
    };
    record.source.stop(now + seconds);
  }

  function activate(plan, buffer) {
    if (active) {
      if (context.currentTime >= active.rampUntil) fadeActive(plan.fadeSeconds);
      else {
        disconnect(active);
        disconnect(fading);
        active = fading = null;
      }
    }
    active = {file: plan.file, buffer, offset: 0, source: null};
    attach(active, fading ? .75 * plan.fadeSeconds : 0);
    onStart(plan.file);
  }

  async function load(file) {
    if (cache.has(file)) return cache.get(file);
    if (failed.has(file)) return null;
    try {
      const asset = manifest.assets[file];
      const url = new URL(asset.url, base);
      url.searchParams.set('v', asset.sha256.slice(0, 16));
      const response = await fetchFile(url, {cache: 'force-cache'});
      if (!response.ok) throw Error('Soundtrack file unavailable');
      const buffer = await context.decodeAudioData(await response.arrayBuffer());
      if (!Number.isFinite(buffer.duration) || buffer.duration <= 0) throw Error('Invalid soundtrack duration');
      cache.set(file, buffer);
      if (cache.size > 2) cache.delete(cache.keys().next().value);
      return buffer;
    } catch {
      failed.add(file);
      return null;
    }
  }

  function suspend() {
    audible = false;
    lastTime = null;
    if (active?.source) {
      active.offset = position(active);
      disconnect(active);
    }
    disconnect(fading);
    fading = null;
  }

  function reset() {
    suspend();
    active = null;
    rebind();
    controller?.reset();
  }

  function rebind() {
    mode = null;
    request = readyPiece = null;
    lastTime = null;
  }

  function update({running, enabled, civilization, epoch, mode: targetMode = 'game'}) {
    if (status === 'unavailable') return false;
    if (!running || !enabled || context.state !== 'running') { suspend(); return true; }
    if (status !== 'ready') return true;
    if (targetMode !== mode) {
      rebind();
      mode = targetMode;
      if (mode === 'menu') menu.reset();
      else controller.reset();
    }
    audible = true;
    const now = context.currentTime;
    const elapsedSeconds = lastTime === null ? 0 : Math.max(0, now - lastTime);
    lastTime = now;
    if (active && !active.source) attach(active);
    // Neutral slot 14 is the own game's western presentation adapter.
    const region = civilization === -1 || civilization === 14 ? 0 : civilizationRegions[civilization];
    const period = epoch >= 0 && epoch < 15 ? epoch < 10 ? 0 : epoch < 13 ? 1 : 2 : undefined;
    const membership = mode === 'menu' ? {region: null, epoch: null}
      : {region: manifest.regionSets[region] ?? null, epoch: manifest.epochSets[period] ?? null};
    if (failed.size) {
      const eligible = mode === 'menu' ? manifest.menu : [...(membership.region?.files ?? []), ...(membership.epoch?.files ?? [])];
      if (eligible.length && eligible.every(file => failed.has(file))) { suspend(); return false; }
    }
    const pending = request ?? readyPiece;
    if (pending && (pending.membership.region !== membership.region || pending.membership.epoch !== membership.epoch)) {
      request = readyPiece = null;
    }
    if (request) return true;
    if (readyPiece) {
      activate(readyPiece.plan, readyPiece.buffer);
      readyPiece = null;
      return true;
    }
    const stream = active ? {positionMs: Math.floor(position(active) * 1000), durationMs: Math.floor(active.buffer.duration * 1000)} : null;
    const plan = mode === 'menu' ? menu.update(stream) : controller.update({membership, elapsedSeconds, stream});
    if (plan?.kind === 'fade') fadeActive(plan.fadeSeconds);
    if (plan?.kind === 'play') {
      const selected = {plan, membership};
      request = selected;
      load(plan.file).then(buffer => {
        if (request !== selected) return;
        request = null;
        if (!buffer) {
          if (mode === 'menu') menu.reset();
          else controller.reset();
          return;
        }
        if (audible) activate(plan, buffer);
        else readyPiece = {...selected, buffer};
      });
    }
    return true;
  }

  return Object.freeze({ready, update, suspend, reset, rebind,
    status: () => ({status, error, file: active?.file ?? null, active: !!active?.source,
      fading: fading?.file ?? null, mode, cached: cache.size, failed: [...failed], sourceState: random.state()})});
}
