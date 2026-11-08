// Authored Farm bridge to the installed helper's placement observations.
export const FARM_INITIAL_YAW = Math.fround(-Math.PI / 4);
const HELPER_TAU = Math.fround(6.283185005187988);
const WRAP_TOLERANCE = Math.fround(1e-5);
const NORMAL_ROTATION_STEP = -FARM_INITIAL_YAW;

export function createFarmAngleMemory() {
  const byType = new Map();
  let active, revision, previousControl, pending;

  function close() {
    active = null;
    revision = 0;
    previousControl = false;
    pending = null;
  }

  return {
    open(type) {
      close();
      if (!byType.has(type)) byType.set(type, { saved: 0, observed: 0, at: 0 });
      active = byType.get(type);
    },
    close,
    reset() { close(); byType.clear(); },
    controlPressed() { previousControl = true; },
    observe(yaw, nextRevision, gameSeconds, now) {
      if (!active || !nextRevision || nextRevision === revision ||
          !(gameSeconds > Math.fround(.1)) || !Number.isFinite(yaw)) return;
      revision = nextRevision;
      active.observed = yaw;
      if (yaw !== FARM_INITIAL_YAW) {
        active.at = Math.floor(now);
        if (pending && Math.abs(Math.fround(Math.abs(active.saved) - Math.abs(yaw))) <= .09) pending = null;
        return;
      }
      const saved = active.saved;
      if (saved === 0 || saved === yaw ||
          Math.abs(Math.fround(HELPER_TAU - Math.fround(Math.abs(yaw) + saved))) < WRAP_TOLERANCE) return;
      if (Math.floor(now) > active.at + 30000) {
        active.saved = 0;
        pending = null;
        return;
      }
      pending = { delta: Math.fround(yaw - saved), tries: 0 };
    },
    input(controlDown, now) {
      const wasDown = previousControl;
      previousControl = controlDown;
      if (!active || !revision) return null;
      if (wasDown) {
        if (!pending) {
          active.saved = active.observed;
          active.at = Math.floor(now);
        }
        return null;
      }
      if (!pending) return null;
      ++pending.tries;
      if (pending.tries > 4) {
        active.saved = 0;
        pending = null;
        return NORMAL_ROTATION_STEP;
      }
      if (pending.tries > 2) pending.delta = Math.fround(active.observed - active.saved);
      return pending.delta;
    },
  };
}
