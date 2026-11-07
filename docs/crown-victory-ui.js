export const CROWN_VICTORY_COUNT = 1;
export const CROWN_VICTORY_TIME = 2;
export const CROWN_VICTORY_CAPTION = 'Kronen-Status';

const rules = Object.freeze({
  'crowns-count': { mode: CROWN_VICTORY_COUNT, defaultTarget: 3, targets: [2, 3] },
  'crowns-time': { mode: CROWN_VICTORY_TIME, defaultTarget: 10, targets: [10, 15, 20, 25, 30] },
});

export function checkCrownVictorySettings(settings) {
  const rule = Object.values(rules).find(row => row.mode === settings?.mode);
  if (!rule?.targets.includes(settings.target)) throw Error('Ungültiges Kronen-Siegziel.');
  return { mode: rule.mode, target: settings.target };
}

export function bindCrownVictoryMenu({ victoryCondition, countRow, count, durationRow, duration, enabled }) {
  const fields = { 'crowns-count': [countRow, count], 'crowns-time': [durationRow, duration] };
  for (const option of victoryCondition.options) {
    if (Object.hasOwn(rules, option.value)) option.hidden = option.disabled = !enabled;
  }
  if (!enabled && Object.hasOwn(rules, victoryCondition.value)) victoryCondition.value = 'conquest';

  let previous;
  function update({ locked = false } = {}) {
    const key = victoryCondition.value;
    const active = enabled && Object.hasOwn(rules, key) ? key : undefined;
    for (const [mode, [row, input]] of Object.entries(fields)) {
      row.hidden = mode !== active;
      input.disabled = row.hidden || locked;
      if (mode === active && active !== previous) input.value = String(rules[mode].defaultTarget);
    }
    previous = active;
    return active !== undefined;
  }

  update();
  return {
    update,
    read() {
      const key = victoryCondition.value;
      if (!Object.hasOwn(rules, key)) return undefined;
      if (!enabled) return null;
      const rule = rules[key];
      const target = rule.targets.find(value => String(value) === fields[key][1].value);
      return target === undefined ? null : { mode: rule.mode, target };
    },
  };
}

export function readCrownVictoryHud(wasm, players, local, { minimized = false, fixedTeamOf } = {}) {
  if (typeof wasm?.crown_victory_status !== 'function') return null;
  const mode = wasm.crown_victory_status(0, 0);
  if (mode !== CROWN_VICTORY_COUNT && mode !== CROWN_VICTORY_TIME) return null;
  const caption = CROWN_VICTORY_CAPTION;
  const rows = [];
  if (minimized) return { caption, rows };

  const required = wasm.crown_victory_status(1, 0);
  if (!Number.isInteger(required) || required <= 0) return null;
  const live = (players ?? []).filter(player => player.alive && Number.isInteger(player.team) && player.team >= 0 && player.team < 4);
  live.sort((a, b) => a.team - b.team);
  for (const player of live) {
    const seat = player.team;
    const progress = wasm.crown_victory_status(2, seat);
    if (!Number.isInteger(progress) || progress < 0) continue;
    const fixedTeam = fixedTeamOf?.(seat);
    const name = player.name || `Spieler ${seat + 1}`;
    rows.push({
      seat,
      name: Number.isInteger(fixedTeam) && fixedTeam >= 0 ? `${name} (Team: ${fixedTeam + 1})` : name,
      status: `Kronen-Fortschritt: ${progress}/${required}`,
      crownMask: mode === CROWN_VICTORY_COUNT ? wasm.crown_victory_status(3, seat) & 7 : 0,
      local: seat === local,
    });
  }
  return { caption, rows };
}
