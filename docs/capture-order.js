import {nativeMetaEpoch} from './original-hud-layout.js';

const artRoot = new URL('./local-original/capture-native/', import.meta.url);
const imageUrl = path => `url("${new URL(path, artRoot).href}")`;
const cursors = {
  valid: `${imageUrl('shared/cursor_capture.png')} 16 16, crosshair`,
  invalid: `${imageUrl('shared/cursor_capture_invalid.png')} 2 2, not-allowed`,
};

export const captureOrderText = Object.freeze({
  label: 'Gebäude erobern',
  description: 'Befiehlt der Einheit, ein feindliches Gebäude zu erobern.',
  shortcut: 'Z',
});

export function createCaptureOrder({canvas, canOrder, pickTarget, issue, originalArt = true,
    appearance, shortcut = captureOrderText.shortcut, fallback = '⚑'}) {
  let armed = false, previousCursor = '', paintButton;

  function cancel() {
    if (!armed) return false;
    armed = false;
    canvas.style.cursor = previousCursor;
    paintButton?.();
    return true;
  }

  function update(point) {
    if (!armed) return false;
    if (!canOrder()) { cancel(); return false; }
    const valid = Number.isFinite(point?.[0]) && Number.isFinite(point?.[1]) && pickTarget(...point) > 0;
    const cursor = appearance?.cursor(valid) ?? (originalArt ? (valid ? cursors.valid : cursors.invalid) : (valid ? 'crosshair' : 'not-allowed'));
    if (canvas.style.cursor !== cursor) canvas.style.cursor = cursor;
    return valid;
  }

  function arm(point) {
    if (!canOrder()) { cancel(); return false; }
    if (!armed) {
      previousCursor = canvas.style.cursor;
      armed = true;
      paintButton?.();
    }
    update(point);
    return true;
  }

  // The original mode stays armed after an invalid click. A dispatched order
  // ends it, including when another mode could normally use Shift to repeat.
  function release(point) {
    if (!update(point)) return 0;
    const ordered = issue(...point);
    if (ordered > 0) cancel();
    return ordered;
  }

  function decorateButton(button, epoch) {
    const theme = nativeMetaEpoch(epoch);
    const suffix = theme === 1 ? '' : `_e${theme}`;
    const art = button.ownerDocument.createElement('span');
    const pictures = appearance
      ? ['idle', 'hover', 'pressed'].map(state => appearance.button(theme, state))
      : originalArt && ['icon_unitact_captureBuilding', 'icon_unitact_captureBld_r', 'icon_unitact_captureBld_p']
        .map(name => imageUrl(`packed7/${name}${suffix}.png`));
    const hasPictures = pictures && pictures.every(Boolean);
    art.style.cssText = 'display:block;width:44px;height:44px;margin:auto;background:center/contain no-repeat;font-size:29px;line-height:44px';
    if (!hasPictures) art.textContent = fallback;
    art.setAttribute('aria-hidden', 'true');
    button.setAttribute('aria-keyshortcuts', shortcut);
    let hovered = false, pressed = false;
    function paint() {
      button.setAttribute('aria-pressed', String(armed));
      if (hasPictures) art.style.backgroundImage = pictures[armed || pressed ? 2 : hovered ? 1 : 0];
    }
    button.addEventListener('pointerenter', () => { hovered = true; paint(); });
    button.addEventListener('pointerleave', () => { hovered = pressed = false; paint(); });
    button.addEventListener('pointerdown', event => {
      if (event.button === 0) { pressed = true; paint(); }
    });
    button.addEventListener('pointerup', () => { pressed = false; paint(); });
    button.addEventListener('pointercancel', () => { pressed = false; paint(); });
    button.append(art);
    paintButton = paint;
    paint();
  }

  return Object.freeze({arm, cancel, active: () => armed, update, release, decorateButton});
}
