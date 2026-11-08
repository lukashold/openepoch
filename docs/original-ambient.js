const f32 = Math.fround;
const piOver2 = f32(Math.PI / 2), piOver4 = f32(Math.PI / 4);
const wordScale = 32769 / 1073741824;
const groups = ['land', 'water', 'forest'];
const climates = ['Temperate', 'Arid', 'Tropical', 'Tundra'];
const manifestHash = '3b0927335f0245bd928206bcf89aa19037f61a7277f2214ccff21595a342cb09';
const floorBits = new DataView(new ArrayBuffer(8));

async function checkedBytes(fetchFile, url, hash) {
  url.searchParams.set('v', hash.slice(0, 16));
  const response = await fetchFile(url);
  if (!response.ok) throw new Error('Missing original ambience');
  const bytes = await response.arrayBuffer();
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  if (Array.from(digest, byte => byte.toString(16).padStart(2, '0')).join('') !== hash) throw new Error('Original ambience checksum differs');
  return bytes;
}

export async function loadOriginalAmbient({enabled, base = import.meta.url, fetchFile = fetch} = {}) {
  if (!enabled) return null;
  try {
    const manifestURL = new URL('./local-original/ambient39/manifest.json', base);
    const catalog = JSON.parse(new TextDecoder().decode(await checkedBytes(fetchFile, manifestURL, manifestHash)));
    if (catalog.format !== 'ee2-native-ambient-audio-v1') return null;
    const read = async asset => {
      const row = catalog.assets[asset];
      if (!row || !/^(?:audio|ambient39)\/[A-Za-z0-9_.-]+$/.test(row.path)) throw new Error('Invalid original ambience path');
      const bytes = await checkedBytes(fetchFile, new URL('../' + row.path, manifestURL), row.sha256);
      if (bytes.byteLength !== row.bytes) throw new Error('Original ambience size differs');
      return bytes;
    };
    return {catalog, read};
  } catch { return null; }
}

// Original 50C300 uses movement categories and the separate forest mask.
// For generated OpenEpoch maps, adapt existing tile classes and actual wood tiles.
export function generatedAmbientCoverage(types, width, height, resourceKind) {
  if (types?.length !== width * height || width <= 0 || height <= 0) return null;
  // The forest plane is independent of the ground movement category.
  const tileFlags = [2, 3, 1, 1, 5, 1];
  const cols = Math.ceil(width / 4), rows = Math.ceil(height / 4);
  const coverage = new Uint8Array(cols * rows);
  for (let y = 0; y < height; y++) {
    for (let x = 0; x < width; x++) {
      const tile = y * width + x, area = (y >> 2) * cols + (x >> 2);
      const flags = tileFlags[types[tile]];
      if (flags === undefined) return null;
      coverage[area] |= flags;
      if (!(coverage[area] & 4) && resourceKind?.(tile) === 2) coverage[area] |= 4;
    }
  }
  return {width, height, cols, rows, coverage};
}

// The forest group's 0050CAE0 callback refreshes its current 4x4 mask every
// 600 wall-clock seconds. Ground and water retain their initial sensor maps.
export function liveGeneratedAmbientCoverage(readTypes, width, height, resourceKind, now) {
  const map = generatedAmbientCoverage(readTypes(), width, height, resourceKind);
  if (!map) return null;
  const {cols, rows, coverage} = map;
  let scannedAt = f32(now);
  map.refreshForest = seconds => {
    if (!(seconds >= scannedAt + 600)) return false;
    const types = readTypes();
    if (types?.length !== width * height) return false;
    for (let by = 0; by < rows; by++) {
      for (let bx = 0; bx < cols; bx++) {
        let hasForest = false;
        for (let y = by * 4; !hasForest && y < Math.min(height, by * 4 + 4); y++) {
          for (let x = bx * 4; x < Math.min(width, bx * 4 + 4); x++) {
            const tile = y * width + x;
            if (types[tile] === 4 || resourceKind?.(tile) === 2) { hasForest = true; break; }
          }
        }
        const area = by * cols + bx;
        coverage[area] = (coverage[area] & 3) | (hasForest ? 4 : 0);
      }
    }
    scannedAt = f32(seconds);
    return true;
  };
  return map;
}

function sourceCell(value) {
  floorBits.setFloat64(0, 103079215104 + value, true);
  return Number(BigInt.asIntN(32, floorBits.getBigUint64(0, true) >> 16n));
}

export function ambientListenerWindow(view, cols, rows) {
  if (!view?.eye || !view.groundForward || !Number.isFinite(view.pitch)) return null;
  const [x, z, y] = view.eye.map(f32);
  const pitch = Math.min(piOver2, Math.max(piOver4, f32(view.pitch)));
  const reach = f32(5 * (piOver2 - pitch));
  const dx = f32(reach * f32(view.groundForward[0])), dy = f32(reach * f32(view.groundForward[1]));
  const radius = 1.25 * Math.min(14, Math.max(7, z));
  return [Math.max(0, Math.trunc(f32((x - radius + dx) * .25))),
    Math.max(0, Math.trunc((y - radius + dy) * .25)),
    Math.min(cols - 1, sourceCell(f32((x + radius + dx) * .25) + .5)),
    Math.min(rows - 1, sourceCell((y + f32(radius) + dy) * .25 + .5))];
}

export function originalAmbientPriorities(map, view, exploration, observer = false) {
  if (!map || !observer && exploration?.length !== map.width * map.height) return null;
  const {width,height,cols,rows,coverage}=map;
  const window = ambientListenerWindow(view, cols, rows);
  if (!window) return null;
  const [x0, y0, x1, y1] = window;
  const counts = [0, 0, 0];
  let seen = 0;
  for (let by = y0; by <= y1; by++) {
    for (let bx = x0; bx <= x1; bx++) {
      if (!observer) {
        let explored = false;
        for (let y = by * 4; !explored && y < Math.min(height, by * 4 + 4); y++) {
          for (let x = bx * 4; x < Math.min(width, bx * 4 + 4); x++) {
            if (exploration[y * width + x] !== 0) { explored = true; break; }
          }
        }
        if (!explored) continue;
      }
      const flags = coverage[by * cols + bx];
      seen++;
      for (let group = 0; group < counts.length; group++) counts[group] += flags >>> group & 1;
    }
  }
  return counts.map((count, group) => {
    const minimum = group === 2 ? 5 : 10;
    return f32(count / seen * (count < minimum ? count * f32(1 / minimum) : 1));
  });
}

// Native selection is inclusive at the cumulative boundary (50BA7B).
export function chooseOriginalAmbient(records, context, random) {
  const eligible = records.filter(record => (record.context & context) === record.context);
  let sum = eligible.reduce((total, record) => total + record.priority, 0);
  if (!sum) return null;
  const choice = random.word15() % sum;
  sum = 0;
  return eligible.find(record => (sum += record.priority) >= choice) ?? null;
}

export function originalAmbientOneShotAt(now, frequency, random) {
  const interval = 60 / frequency;
  const fraction = random.word15() * wordScale;
  return f32(now + interval * (.5 + fraction));
}

export function createOriginalAmbientPlayback({presentation, context, destination, random}) {
  const {catalog, read} = presentation;
  const decoded = new Map(), profiles = new Map(), channels = new Set();
  const states = new Map();
  let session = null, climate = null, last = null, ready = false;

  function prepare(climate, weather) {
    const profile = catalog.profiles[climate];
    if (!profile) return null;
    const weatherSet = profile.weather[weather];
    const key = climate + ':' + (weatherSet ?? '');
    if (profiles.has(key)) return profiles.get(key);
    const status = {ready:false};
    profiles.set(key, status);
    const records = groups.map(group => profile[group]);
    if (weatherSet) records.push(weatherSet);
    const names = new Set(records.flatMap(name => [...catalog.sets[name].loops, ...catalog.sets[name].shots].map(cue => cue.asset)));
    const load = name => {
      if (!decoded.has(name)) decoded.set(name, read(name).then(bytes => context.decodeAudioData(bytes)).catch(() => null));
      return decoded.get(name);
    };
    Promise.all(Array.from(names, name => load(name).then(buffer => [name, buffer]))).then(pairs => {
      if (pairs.every(([, buffer]) => buffer)) { status.buffers = new Map(pairs); status.ready = true; }
    });
    return status;
  }

  function ramp(channel, target, duration) {
    const at = context.currentTime;
    const {from,to,start,end} = channel;
    const current = at >= end ? to : from + (to - from) * Math.max(0, at - start) / (end - start);
    const gain = channel.gain.gain;
    gain.cancelScheduledValues(at);
    gain.setValueAtTime(duration ? current : target, at);
    if (duration) gain.linearRampToValueAtTime(target, at + duration);
    Object.assign(channel, {from:duration ? current : target, to:target, start:at, end:at + duration});
  }

  function sound(buffer, cue, mix, fade) {
    const node = context.createBufferSource(), gain = context.createGain();
    node.buffer = buffer;node.loop = cue.sourceLoop;
    node.connect(gain);gain.connect(destination);
    const channel = {node, gain, from:0, to:0, start:0, end:0};
    channels.add(channel);
    node.onended = () => { channels.delete(channel);node.disconnect();gain.disconnect(); };
    ramp(channel, f32(cue.gain * mix), fade);
    node.start();
    return channel;
  }

  function retire(state) {
    if (!state.loop) return;
    ramp(state.loop, 0, 3);
    state.loop.node.stop(context.currentTime + 3);
    state.loop = null;
  }

  function reset() {
    for (const {node,gain} of channels) {
      node.stop();node.disconnect();gain.disconnect();
    }
    channels.clear();states.clear();session = null;last = null;ready = false;
  }

  function frame({world, climate:climateId, now, priorities, weather = 'Normal', running, soundContext = 0, refreshForest}) {
    const nextClimate = climates[climateId];
    if (world !== session || nextClimate !== climate || last !== null && now < last) {
      reset();session = world;
      if (nextClimate !== climate) { decoded.clear();profiles.clear();climate = nextClimate; }
    }
    const dt = last === null ? 0 : f32(f32(now) - f32(last));
    last = now;
    if (!running) {
      for (const state of states.values()) retire(state);
      states.clear();
      return ready;
    }
    const profile = catalog.profiles[climate];
    const status = prepare(climate, weather);
    if (!priorities || !status?.ready) return ready;
    ready = true;
    const selections = [...Object.entries(profile.weather).map(([condition, name]) => [name, condition === weather ? 1 : 0]),
      ...groups.map((group, index) => [profile[group], priorities[index]])];
    for (const [name, priority] of selections) {
      const set = catalog.sets[name], cutoff = f32(set.cutoff), minimum = f32(set.minimum);
      let state = states.get(name);
      if (!(priority >= cutoff)) {
        if (state) { retire(state);states.delete(name); }
        continue;
      }
      if (!state) {
        state = {delay:0, loop:null, cue:null, shotAt:originalAmbientOneShotAt(now, set.frequency, random)};
        states.set(name, state);
      }
      // The source priority query precedes this active sensor callback.
      if (name === profile.forest) refreshForest?.(now);
      const mix = f32(minimum + (priority - cutoff) / (1 - cutoff) * (1 - minimum));
      if (state.loop && state.mix !== mix) {
        const channel = state.loop;
        ramp(channel, f32(state.cue.gain * mix), Math.max(0, channel.end - context.currentTime));
      }
      state.mix = mix;
      if (state.delay > 0) state.delay = f32(state.delay - dt);
      if (!state.loop || state.delay <= 0) {
        const cue = chooseOriginalAmbient(set.loops, soundContext, random);
        if (cue !== state.cue) {
          retire(state);
          state.cue = cue;
          if (cue) { state.loop = sound(status.buffers.get(cue.asset), cue, mix, 2);state.delay = 15; }
        }
      }
      if (now >= state.shotAt) {
        const cue = chooseOriginalAmbient(set.shots, soundContext, random);
        if (cue) sound(status.buffers.get(cue.asset), cue, mix, 0);
        state.shotAt = originalAmbientOneShotAt(now, set.frequency, random);
      }
    }
    return true;
  }

  return {frame, reset, status:() => ({climate, ready, groups:states.size, channels:channels.size})};
}
