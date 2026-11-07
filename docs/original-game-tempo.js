// Installed GameSpeedType entries and the request clamp from 005A7E70.
const f32 = Math.fround;
export const ORIGINAL_SPEEDS = Object.freeze([.7, 1, 1.3, 1.6].map(f32));
export const NORMAL_TEMPO = Object.freeze({preset: 1, locked: true});
const MINIMUM = f32(.05);
const MAXIMUM = ORIGINAL_SPEEDS[3];

export function checkOriginalTempo(value) {
  if (!value || !Number.isInteger(value.preset) || value.preset < 0 ||
      value.preset >= ORIGINAL_SPEEDS.length || typeof value.locked !== 'boolean') {
    throw Error('Ungültiges Spieltempo');
  }
  return {preset: value.preset, locked: value.locked};
}

export function originalStartingSpeed(tempo) {
  return ORIGINAL_SPEEDS[tempo.preset];
}

export function changeOriginalSpeed(current, factor) {
  return Math.max(MINIMUM, Math.min(MAXIMUM, f32(current * factor)));
}

export function validOriginalSpeed(speed) {
  return Number.isFinite(speed) && f32(speed) === speed && speed >= MINIMUM && speed <= MAXIMUM;
}

export function loadOriginalTempo(value) {
  if (value === undefined) return {...NORMAL_TEMPO, requested: 1};
  const tempo = checkOriginalTempo(value);
  const {requested} = value;
  if (!validOriginalSpeed(requested) ||
      (tempo.locked && requested !== originalStartingSpeed(tempo))) {
    throw Error('Ungültiges Spieltempo im Spielstand');
  }
  return {...tempo, requested};
}
