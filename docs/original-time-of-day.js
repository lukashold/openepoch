const minutes = [2, 5, 10, 15];
const word = new DataView(new ArrayBuffer(4));

function positiveFloatTowardZero(value) {
  word.setFloat32(0, value, true);
  if (word.getFloat32(0, true) > value) word.setUint32(0, word.getUint32(0, true) - 1, true);
  return word.getFloat32(0, true);
}

export const timeOfDayPeriods = Object.freeze(['Morning', 'Afternoon', 'Evening', 'Night']);

export function checkTimeOfDayMode(value) {
  if (!Number.isInteger(value) || value < 0 || value > 7) throw new RangeError('Invalid time of day');
  return value;
}

export function resolveTimeOfDay(value, word15) {
  return value === -2 ? word15() & 3 : checkTimeOfDayMode(value);
}

export function readTimeOfDayState(state, mode = state?.mode) {
  checkTimeOfDayMode(mode);
  if (state?.mode !== mode || ![0, 1, 2, 3].includes(state.phase) || ![0, 1, 2, 3].includes(state.displayed) ||
      typeof state.pendingWorld !== 'boolean' || typeof state.enabled !== 'boolean' ||
      ![state.deadline, state.lastTransition].every(value => Number.isFinite(value) && value >= 0 && Math.fround(value) === value) ||
      state.interval !== (mode >= 4 && !state.pendingWorld ? minutes[mode - 4] * 60 : 10) ||
      (mode < 4 && (state.enabled || state.displayed !== mode || state.deadline !== 0 || state.lastTransition !== 0)) ||
      (state.pendingWorld && (state.phase !== 0 || state.enabled || state.deadline !== 0 || state.lastTransition !== 0 ||
        state.displayed !== (mode < 4 ? mode : 1)))) throw new RangeError('Invalid time-of-day state');
  const {phase, displayed, interval, deadline, lastTransition, enabled, pendingWorld} = state;
  return {mode, phase, displayed, interval, deadline, lastTransition, enabled, pendingWorld};
}

// Sample the previous simulation time once per simulation step, at the same
// boundary as the installed UP hook. The display itself does not drive it.
export function createTimeOfDayClock(state = null) {
  let {mode, phase, displayed, interval, deadline, lastTransition, enabled, pendingWorld} = state === null ?
    {mode: 1, phase: 0, displayed: 1, interval: 10, deadline: 0, lastTransition: 0, enabled: false, pendingWorld: false} :
    readTimeOfDayState(state);

  function begin(choice) {
    mode = checkTimeOfDayMode(choice);
    displayed = mode < 4 ? mode : 1;
    pendingWorld = true;
  }

  function step(previousElapsedSeconds) {
    const now = Math.fround(previousElapsedSeconds);
    if (!Number.isFinite(now) || now < 0) throw new RangeError('Invalid simulation time');
    if (pendingWorld) {
      phase = mode;
      pendingWorld = false;
    }
    if (phase >= 4) {
      interval = minutes[phase - 4] * 60;
      enabled = true;
      phase = 3;
      deadline = 0;
    } else if (enabled && now > deadline) {
      deadline = positiveFloatTowardZero(now + interval);
      lastTransition = now;
      phase = (phase + 1) & 3;
      displayed = phase;
      return phase;
    } else if (enabled && lastTransition > now) {
      deadline = 0;
      enabled = false;
    }
    return null;
  }

  return Object.freeze({begin, step, snapshot: () => ({mode, phase, displayed, interval, deadline, lastTransition, enabled, pendingWorld})});
}
