// eClimate values and display order from the installed UP map-settings combo.
export const CLIMATE_NAMES = Object.freeze(['Temperate', 'Arid', 'Tropical', 'Tundra']);
const DEFAULT_CLIMATES = Object.freeze([0]);
const ALL_CLIMATES = Object.freeze(CLIMATE_NAMES.map((_, value) => value));

export async function loadSkirmishClimates(enabled, fetchFile = globalThis.fetch) {
  if (!enabled) return null;
  try {
    const response = await fetchFile(new URL('./local-original/skirmish-climates/manifest.json', import.meta.url));
    if (!response.ok) return null;
    const manifest = await response.json();
    if (manifest.format !== 'ee2-skirmish-climates-v1' || !CLIMATE_NAMES.every((name, value) => {
      const profile = manifest.profiles?.[name];
      return profile?.value === value && profile.terrain && profile.ground && profile.body;
    })) return null;
    return manifest.profiles;
  } catch {
    return null;
  }
}

export function supportedSkirmishClimates(profiles, terrain, ground, models) {
  if (!profiles) return DEFAULT_CLIMATES;
  return Object.freeze(CLIMATE_NAMES.flatMap((name, value) => value === 0 ||
    [terrain, ground, models].every(renderer => renderer.seasonClimates.includes(name)) ? [value] : []));
}

export function checkedClimate(value, supported = DEFAULT_CLIMATES) {
  if (value === undefined) return 0;
  if (!Number.isInteger(value) || !supported.includes(value)) throw new RangeError('Ungültige Klimaeinstellung.');
  return value;
}

export function savedClimate(record, supported) {
  const climate = checkedClimate(record.climate, ALL_CLIMATES);
  return supported.includes(climate) ? climate : 0;
}

export function peerClimates(values) {
  if (values === undefined) return DEFAULT_CLIMATES;
  if (!Array.isArray(values) || values[0] !== 0 || values.length > CLIMATE_NAMES.length ||
      values.some((value, index) => !Number.isInteger(value) || value < 0 || value >= CLIMATE_NAMES.length ||
        (index > 0 && value <= values[index - 1]))) {
    throw new RangeError('Ungültige Klimaliste.');
  }
  return values;
}
