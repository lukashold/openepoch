export const KOTH_DURATIONS = Object.freeze([2, 5, 10]);
export const KOTH_DEFAULT_MINUTES = 10;

export function bindKothMenu({victoryCondition, durationRow, duration, enabled}) {
  const option = Array.from(victoryCondition.options).find(row => row.value === 'koth');
  option.hidden = option.disabled = !enabled;
  if (!enabled && victoryCondition.value === 'koth') victoryCondition.value = 'conquest';
  let previouslyShown = false;
  function update() {
    const shown = enabled && victoryCondition.value === 'koth';
    if (shown && !previouslyShown) duration.value = String(KOTH_DEFAULT_MINUTES);
    previouslyShown = shown;
    durationRow.hidden = duration.disabled = !shown;
    return shown;
  }
  update();
  return {
    update,
    readMinutes() {
      if (!enabled || victoryCondition.value !== 'koth') return null;
      return KOTH_DURATIONS.find(minutes => String(minutes) === duration.value) ?? null;
    },
  };
}

// The native HUD uses signed IDIV 60 and %.2d:%.2d, with no clamp or hour field.
export function formatKothCountdown(seconds) {
  if (!Number.isInteger(seconds) || seconds < -2147483648 || seconds > 2147483647) return '';
  const decimal = value => (value < 0 ? '-' : '') + String(Math.abs(value)).padStart(2, '0');
  return decimal(Math.trunc(seconds / 60)) + ':' + decimal(seconds % 60);
}

export function formatKothStatus(holder, seconds, template = 'König: %PLAYER% Countdown: %REMAININGTIME%') {
  const remaining = formatKothCountdown(seconds);
  if (typeof holder !== 'string' || !holder || !remaining) return '';
  return template.replace(/%PLAYER%|%REMAININGTIME%/g, token => token === '%PLAYER%' ? holder : remaining);
}
