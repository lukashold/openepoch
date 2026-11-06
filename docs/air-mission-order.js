const none = 0xffffffff;
const waypointLimit = 5;
const artDirectory = 'local-original/air-mission-ui/';
const artHash = 'a1b5454a02fafa356bb3272e3d6960b44845c472432e822bab1c3253fae87e93';
const states = ['idle', 'hover', 'pressed', 'hoverPressed'];

export const airActions = Object.freeze({
  set: {label:'Missionsflagge festlegen', key:'R', glyph:'⚑'},
  cancel: {label:'Mission abbrechen', command:1, glyph:'×'},
  scramble: {label:'Alarmstart', key:'U', command:2, glyph:'⬆'},
  launch: {label:'Der Mission Flugzeuge hinzufügen', command:3, rightCommand:5, glyph:'✈'},
  recall: {label:'Zurück zum Stützpunkt', command:4, glyph:'↶'},
});
const airportTrainingKeys = new Map([[37,'F'], [39,'F'], [44,'F'], [83,'T'], [38,'B'], [40,'B']]);

export function supportsAirMission(wasm) {
  return ['selected_actor_handle', 'air_mission_status', 'air_mission_points_ptr', 'air_mission_target_at',
    'cmd_air_mission_route', 'cmd_air_mission_action'].every(key => typeof wasm?.[key] === 'function');
}

export function airMissionSelection(wasm) {
  const handle = wasm.selected_actor_handle() >>> 0;
  if (handle === none) return null;
  const field = id => wasm.air_mission_status(handle, id);
  const kind = field(0);
  if (kind === 1) return {handle, kind, count:field(1), max:field(2), mission:!!field(3), auto:!!field(4), complete:!!field(6)};
  if (kind === 2) return {handle, kind, count:field(1), max:field(2), mission:!!field(3), home:!!field(5)};
  return null;
}

export function airMissionActions(status) {
  if (status?.kind === 2) return status.home ? [{role:'recall', ready:true}] : [];
  if (status?.kind !== 1) return [];
  const actions = [{role:'set', ready:true}];
  if (status.complete) actions.push({role:'scramble', ready:status.count > 0},
    {role:'launch', ready:status.mission && status.count > 0}, {role:'recall', ready:true});
  if (status.mission) actions.push({role:'cancel', ready:true});
  return actions;
}

export function airMissionTrainingKey(status, action) {
  return status?.kind === 1 && action?.k === 't' ? airportTrainingKeys.get(action.id) : undefined;
}

export async function loadAirMissionArt({enabled, profile = 'packed7', fetchFile = globalThis.fetch,
  base = new URL('./', import.meta.url), objectURL = URL.createObjectURL.bind(URL)} = {}) {
  if (!enabled || !['packed5', 'packed7'].includes(profile)) return null;
  async function read(path, expected) {
    const url = new URL(path, base);
    url.searchParams.set('v', expected.slice(0, 16));
    const response = await fetchFile(url);
    if (!response.ok) throw Error('Air mission art unavailable');
    const bytes = await response.arrayBuffer();
    const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
    if (Array.from(digest, byte => byte.toString(16).padStart(2, '0')).join('') !== expected) throw Error('Air mission art checksum differs');
    return bytes;
  }
  try {
    const manifest = JSON.parse(new TextDecoder().decode(await read(artDirectory + 'manifest.json', artHash)));
    if (manifest.format !== 'ee2-native-air-mission-ui-v1') return null;
    const cached = new Map();
    const records = await Promise.all(Object.keys(airActions).map(async role => {
      const buttons = await Promise.all(states.map(async state => {
        const {url, sha256} = manifest.profiles[profile].actions[role][state];
        if (!url.startsWith(artDirectory + profile + '/') || !/^[\w./-]+\.png$/.test(url) || url.includes('..')) throw Error('Invalid air mission art path');
        if (!cached.has(url)) cached.set(url, read(url, sha256).then(bytes => new Blob([bytes], {type:'image/png'})));
        return [state, await cached.get(url)];
      }));
      return [role, Object.fromEntries(buttons)];
    }));
    const urls = new Map();
    const images = Object.fromEntries(records.map(([role, images]) => [role,
      Object.fromEntries(Object.entries(images).map(([state, blob]) => {
        if (!urls.has(blob)) urls.set(blob, objectURL(blob));
        return [state, urls.get(blob)];
      }))]));
    return {images, labels:manifest.labels};
  } catch {
    return null;
  }
}

export function createAirMissionOrder({wasm, canvas, caption, mapSize, host, onChange = () => {}}) {
  let selected = none, oldCursor = '';
  const points = [];
  const active = () => selected !== none;
  function refresh() {
    caption.hidden = !active();
    caption.textContent = active() ? `Missionsflagge · Wegpunkte ${points.length}/${waypointLimit} · Rechtsklick zum Abschließen` : '';
    onChange();
  }
  function cancel() {
    if (!active()) return false;
    selected = none;
    points.length = 0;
    canvas.style.cursor = oldCursor;
    refresh();
    return true;
  }
  function validate() {
    if (!active()) return false;
    if (host() !== selected) { cancel(); return false; }
    return true;
  }
  function arm() {
    const handle = host();
    if (handle === none) return false;
    if (!active()) oldCursor = canvas.style.cursor;
    selected = handle;
    points.length = 0;
    canvas.style.cursor = 'crosshair';
    refresh();
    return true;
  }
  function commit(target = none) {
    if (!validate()) return 0;
    let ordered = 0;
    if (points.length || target !== none) {
      const ptr = wasm.air_mission_points_ptr() >>> 0, buffer = wasm.memory.buffer;
      if (ptr % 4 === 0 && ptr + waypointLimit * 8 <= buffer.byteLength) {
        const output = new Float32Array(buffer, ptr, waypointLimit * 2);
        points.forEach((point, index) => output.set(point, index * 2));
        ordered = wasm.cmd_air_mission_route(points.length, target);
      }
    }
    cancel();
    return ordered;
  }
  function release(point) {
    if (!validate() || point?.length !== 2) return 0;
    const location = point.map(Math.fround);
    if (!location.every(value => Number.isFinite(value) && value >= 0 && value < mapSize)) return 0;
    const target = wasm.air_mission_target_at(...location) >>> 0;
    if (target !== none) return commit(target);
    points.push(location);
    if (points.length === waypointLimit) return commit();
    refresh();
    return 0;
  }
  return {active, validate, arm, cancel, release, finish:() => commit()};
}

export function decorateAirMissionButton(button, role, {art, active, auto} = {}) {
  const action = airActions[role], label = art?.labels[role], images = art?.images[role];
  const frame = button.ownerDocument.createElement('span');
  frame.className = 'air-mission-icon';
  frame.setAttribute('aria-hidden', 'true');
  if (!images) frame.textContent = action.glyph;
  if (action.key) button.setAttribute('aria-keyshortcuts', action.key);
  if (role === 'set' || role === 'launch') button.setAttribute('aria-pressed', String(role === 'set' ? !!active : !!auto));
  if (role === 'launch') button.classList.toggle('air-mission-auto', !!auto);
  button.title = label?.tooltip || action.label;
  button.append(frame);
  if (images) {
    let hovered = false, pressed = false;
    function paint() {
      const state = pressed || active ? hovered ? 'hoverPressed' : 'pressed' : hovered ? 'hover' : 'idle';
      frame.style.backgroundImage = `url("${images[state]}")`;
    }
    button.addEventListener('pointerenter', () => { hovered = true; paint(); });
    button.addEventListener('pointerleave', () => { hovered = pressed = false; paint(); });
    button.addEventListener('pointerdown', event => { if (!event.button) { pressed = true; paint(); } });
    button.addEventListener('pointerup', () => { pressed = false; paint(); });
    button.addEventListener('pointercancel', () => { pressed = false; paint(); });
    paint();
  }
  const text = button.ownerDocument.createElement('small');
  text.textContent = label?.label || action.label;
  button.append(text);
}
