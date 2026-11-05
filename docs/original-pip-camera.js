import {normalizeOriginalHeading, ORIGINAL_CAMERA_PROFILE} from './original-camera.js';

const f32 = Math.fround;
const PAN_RATE = f32(.6); // 007AE530, B48028
const WHEEL_RATE = f32(.04); // 0051D9C0 and 0051E0A0, C0F9C0
const REPEAT_SECONDS = f32(.3); // 007CF520, C5C8DC
const CYCLE_SECONDS = 1.5; // 007CF130, C5C8C8
const BOOKMARKS = 7; // Flare (0) and six numbered positions
const RAD = f32(.01745329238474369);

// PIPCAM constructor 0051CFB0 and installed myconfig.cfg MINFOV/MAXFOV.
export const ORIGINAL_PIP_CAMERA_PROFILE = Object.freeze({...ORIGINAL_CAMERA_PROFILE,
  minDistance: 10, maxDistance: 25, defaultFraction: f32(.7), minHeight: 2,
  minPitch: f32(Math.PI / 12), kneePitch: f32(Math.PI / 6), maxPitch: f32(Math.PI / 4),
  parsedKneeDistance: .25, minFov: f32(f32(23.07) * RAD), maxFov: f32(f32(47.47) * RAD),
});

function checkedPose({x, y, heading, nativeFraction}) {
  if (![x, y, heading, nativeFraction].every(Number.isFinite)) throw new RangeError('Invalid PIP pose');
  return {x: f32(x), y: f32(y), heading: normalizeOriginalHeading(heading), nativeFraction: Math.max(0, Math.min(1, f32(nativeFraction)))};
}

// 007ADE90 checks the content viewport (inclusive left/top, exclusive
// right/bottom). This differs from the map's own point conversion bounds.
export function originalPipContains({x, y, width, height}, clientX, clientY) {
  return x <= clientX && clientX < x + width && y <= clientY && clientY < y + height;
}

export function createOriginalPipCamera(initial) {
  let pose = checkedPose(initial), current = -1, cycling = false, remaining = CYCLE_SECONDS;
  let previousKey = -1, previousTime = -1;
  const bookmarks = Array(BOOKMARKS).fill(null);
  const copy = () => ({...pose});
  const changeSelection = slot => {
    if (slot === current) return;
    current = slot;
    if (slot !== -1 && cycling) remaining = CYCLE_SECONDS;
  };
  const checkedSlot = (slot, allowFlare = true) => {
    if (!Number.isInteger(slot) || slot < (allowFlare ? 0 : 1) || slot >= BOOKMARKS) throw new RangeError('Invalid PIP bookmark slot');
  };
  const moveFocus = (x, y) => {
    if (![x, y].every(Number.isFinite)) throw new RangeError('Invalid PIP focus');
    pose.x = f32(x);
    pose.y = f32(y);
  };
  const choose = slot => {
    if (!bookmarks[slot]) return false;
    changeSelection(slot);
    return true;
  };
  function put(slot, {main, fullmap, unit = null}) {
    if (fullmap && unit === null) return false;
    const camera = checkedPose(main);
    if (unit !== null && (!Number.isInteger(unit.handle) || unit.handle < 0 || unit.handle >= 0xffffffff)) {
      throw new RangeError('Invalid PIP unit bookmark');
    }
    bookmarks[slot] = {camera, handle: unit?.handle ?? null};
    return choose(slot);
  }

  return Object.freeze({
    pose: copy,
    snapshot: () => ({pose: copy(), current, cycling, remaining, bookmarks: structuredClone(bookmarks)}),
    // 00760F40 places the preview while the map is held. Main camera and
    // inclusive full-map coordinate conversion are owned by the caller.
    focusMap: moveFocus,
    // 007AE530 performs one horizontal and one vertical source camera move.
    // Browser Z is the negative of native Y. Positions round at each write.
    drag(dx, dy) {
      if (![dx, dy].every(Number.isInteger)) throw new RangeError('Expected native cursor pixel offsets');
      const heading = f32(ORIGINAL_CAMERA_PROFILE.nativeDefaultHeading + pose.heading);
      const sin = f32(Math.sin(heading)), cos = f32(Math.cos(heading));
      if (dx) moveFocus(pose.x + f32(dx * PAN_RATE * sin), pose.y + f32(dx * PAN_RATE * cos));
      if (dy) moveFocus(pose.x + f32(-dy * PAN_RATE * cos), pose.y + f32(dy * PAN_RATE * sin));
      changeSelection(-1);
      return copy();
    },
    // 0051E0A0 supplies native accumulated wheel units, not DOM pixel deltas.
    // The caller owns visibility and 007ADE90 viewport hit-testing.
    wheel(nativeUnits) {
      if (!Number.isInteger(nativeUnits)) throw new RangeError('Invalid PIP wheel input');
      const change = f32(-nativeUnits * WHEEL_RATE * .5 / ORIGINAL_PIP_CAMERA_PROFILE.maxDistance);
      pose.nativeFraction = Math.max(0, Math.min(1, f32(pose.nativeFraction + change)));
      return copy();
    },
    setBookmark(slot, values) {
      checkedSlot(slot, false);
      return put(slot, values);
    },
    selectBookmark(slot) {
      checkedSlot(slot);
      return choose(slot);
    },
    selectOrSet(slot, time, values) {
      checkedSlot(slot, false);
      if (!Number.isFinite(time) || time < 0) throw new RangeError('Invalid native input clock');
      if (!bookmarks[slot]) return put(slot, values) ? {action: 'set'} : null;
      const repeated = slot === previousKey && time - previousTime <= REPEAT_SECONDS;
      previousTime = f32(time);
      previousKey = repeated ? -1 : slot;
      if (repeated) {
        changeSelection(-1);
        return {action: 'main-focus', x: pose.x, y: pose.y};
      }
      choose(slot);
      return {action: 'select'};
    },
    clearBookmark(slot) {
      checkedSlot(slot, false);
      if (!bookmarks[slot]) return false;
      bookmarks[slot] = null;
      return true;
    },
    setFlare(camera) {
      bookmarks[0] = camera === null ? null : {camera: checkedPose(camera), handle: null};
    },
    toggleCycle() {
      cycling = !cycling;
      if (cycling) remaining = CYCLE_SECONDS;
      return cycling;
    },
    update(delta, resolveUnit) {
      if (!Number.isFinite(delta) || delta < 0 || typeof resolveUnit !== 'function') throw new RangeError('Invalid PIP update');
      for (const [slot, entry] of bookmarks.entries()) {
        if (!entry || entry.handle === null) continue;
        const unit = resolveUnit(entry.handle);
        if (!unit) {
          bookmarks[slot] = null;
        } else {
          entry.camera.x = f32(unit.x);
          entry.camera.y = f32(unit.y);
        }
      }
      if (cycling) {
        remaining = f32(remaining - f32(delta));
        if (current === -1 || !bookmarks[current] || remaining <= 0) {
          remaining = CYCLE_SECONDS;
          for (let offset = 1; offset <= BOOKMARKS; offset++) {
            const slot = (current + offset) % BOOKMARKS;
            if (bookmarks[slot]) { current = slot; break; }
          }
        }
      } else if (current !== -1 && !bookmarks[current]) {
        changeSelection(-1);
      }
      if (current !== -1 && bookmarks[current]) pose = {...bookmarks[current].camera};
      return copy();
    },
  });
}
