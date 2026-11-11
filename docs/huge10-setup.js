import {CIVILIZATIONS, RANDOM_CIVILIZATION, civilizationChoice} from './civilization-setup.js?v=native67';

export const HUGE10_SEATS = 10;
export const HUGE10_SEED = 1844288443;
// Compatible source RNG input for the independently archived seat permutation.
export const HUGE10_SHUFFLE_STATE = 2394123698;
const INITIAL_CIVILIZATIONS = [4,11,10,3,0,5,7,8,2,13];

export function huge10Palette(wasm, family) {
  return Array.from({length:HUGE10_SEATS}, (_, seat) => {
    const word = wasm.huge10_team_rgb(family, seat) >>> 0;
    if (word > 0xffffff) throw Error('Ungültige Pangea-Spielerfarbe.');
    return [(word >>> 16) / 255, ((word >>> 8) & 255) / 255, (word & 255) / 255];
  });
}

export function createHuge10CivilizationSetup(document, anchor) {
  const section = document.createElement('fieldset');
  section.id = 'civilization-options';
  const legend = document.createElement('legend');
  legend.textContent = 'Völker';
  section.append(legend);
  const entries = [...CIVILIZATIONS, {id:RANDOM_CIVILIZATION, name:'Zufällig'}];
  const selects = Array.from({length:HUGE10_SEATS}, (_, seat) => {
    const row = document.createElement('div');
    row.className = 'row';
    const label = document.createElement('label');
    label.htmlFor = 'civilization-' + seat;
    label.textContent = seat ? 'Spieler ' + (seat + 1) : 'Dein Volk';
    const select = document.createElement('select');
    select.id = label.htmlFor;
    for (const {id,name} of entries) {
      const option = document.createElement('option');
      option.value = String(id);
      option.textContent = name;
      select.append(option);
    }
    select.value = String(INITIAL_CIVILIZATIONS[seat]);
    row.append(label, select);
    section.append(row);
    return select;
  });
  anchor.before(section);
  const choice = seat => civilizationChoice(Number(selects[seat].value));
  return Object.freeze({enabled:true, choice, choices:()=>selects.map((_, seat) => choice(seat))});
}

export function resolveHuge10Civilizations(choices, random = () => Math.floor(Math.random() * CIVILIZATIONS.length)) {
  if (choices.length !== HUGE10_SEATS) throw Error('Zehn Pangea-Spieler werden benötigt.');
  return choices.map(choice => {
    const value = civilizationChoice(choice);
    return value === RANDOM_CIVILIZATION ? random() : value;
  });
}

export function applyHuge10Settings(wasm, {civilizations, popLimit, supplies, epoch, finalEpoch}) {
  if (civilizations.length !== HUGE10_SEATS || civilizations.some(civ =>
    !Number.isInteger(civ) || civ < 0 || civ >= CIVILIZATIONS.length)) {
    throw Error('Zehn gültige Pangea-Völker werden benötigt.');
  }
  const low = civilizations.slice(0,8).reduce((word, civ, seat) => word | (civ << (4 * seat)), 0);
  const high = civilizations[8] | (civilizations[9] << 4);
  if (wasm.configure_match_huge10(low, high, popLimit, supplies, epoch, finalEpoch) !== 1) {
    throw Error('Die Einstellungen konnten für Pangea mit zehn Spielern nicht übernommen werden.');
  }
}
