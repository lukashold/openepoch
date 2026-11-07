const MODE = Object.freeze({capitols: 5, 'allied-capitols': 6});
const MINUTES = [2, 5, 10];
export const CAPITOLS_CAPTION = 'Kapitole-Status';

export function bindCapitolsMenu({victory, durationRow, duration, enabled}) {
  for (const option of victory.options) {
    if (Object.hasOwn(MODE, option.value)) option.hidden = option.disabled = !enabled;
  }
  if (!enabled && Object.hasOwn(MODE, victory.value)) victory.value = 'conquest';
  let previous;
  function update({locked = false} = {}) {
    const mode = enabled && Object.hasOwn(MODE, victory.value) ? MODE[victory.value] : undefined;
    if (mode && mode !== previous) duration.value = '10';
    previous = mode;
    durationRow.hidden = !mode;
    duration.disabled = !mode || locked;
    return mode;
  }
  update();
  return {
    update,
    read() {
      if (!Object.hasOwn(MODE, victory.value)) return undefined;
      const minutes = MINUTES.find(value => String(value) === duration.value);
      return enabled && minutes !== undefined ? {mode: MODE[victory.value], minutes} : null;
    },
  };
}

export function readCapitolHud(wasm, players, local, allianceCountdown) {
  const mode = wasm.capitol_status?.(0, 0);
  if (mode !== 5 && mode !== 6) return null;
  const rows = (players ?? []).flatMap(player => {
    if (!player.alive || !Number.isInteger(player.team) || player.team < 0 || player.team > 3) return [];
    const seat = player.team, seconds = wasm.capitol_status(2, seat);
    if (seconds < 0) return [];
    return [{seat, name: player.name || `Spieler ${seat + 1}`, status: `Kapitol-Countdown: ${seconds}`,
      crownMask: 0, local: seat === local}];
  }).sort((a, b) => a.seat - b.seat);
  if (allianceCountdown >= 0) rows.unshift({seat: -1, name: '', crownMask: 0, local: false,
    status: `Bündnisse werden in ${allianceCountdown} Sekunden aufgelöst!`});
  return {caption: CAPITOLS_CAPTION, rows};
}
