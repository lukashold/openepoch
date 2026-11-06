import {nativeMetaEpoch} from './original-hud-layout.js';

export const TRADE_NONE = 0xffffffff;
export const tradeRouteText = Object.freeze({label: 'Handelsroute', title: 'Handelsroute etablieren.',
  home: 'Anfang', destination: 'Ziel', shortcut: 'Shift+T'});

export function supportsTradeRoute(wasm) {
  return ['trade_selected_count', 'trade_endpoint_at', 'cmd_trade_route'].every(key => typeof wasm?.[key] === 'function');
}

export function createTradeRoute({canvas, caption, canOrder, selectionKey, pick, issue, homeValid = () => true, art = null}) {
  let armed = false, home = TRADE_NONE, selected, oldCursor = '', repaint;

  function cancel() {
    if (!armed) return false;
    armed = false;
    home = TRADE_NONE;
    canvas.style.cursor = oldCursor;
    caption.hidden = true;
    repaint?.();
    return true;
  }

  function validate() {
    if (!armed) return false;
    if (!canOrder() || selectionKey() !== selected) { cancel(); return false; }
    if (home !== TRADE_NONE && !homeValid(home)) {
      home = TRADE_NONE;
      caption.textContent = tradeRouteText.home;
      canvas.style.cursor = art?.cursor(false) ?? 'not-allowed';
    }
    return true;
  }

  function update(point, screen) {
    if (!validate()) return TRADE_NONE;
    const raw = Number.isFinite(point?.[0]) && Number.isFinite(point?.[1]) ? pick(...point, home) : TRADE_NONE;
    const endpoint = Number.isInteger(raw) ? raw >>> 0 : TRADE_NONE;
    const valid = endpoint !== TRADE_NONE && endpoint !== home;
    canvas.style.cursor = art?.cursor(valid) ?? (valid ? 'crosshair' : 'not-allowed');
    const label = home === TRADE_NONE ? tradeRouteText.home : tradeRouteText.destination;
    if (caption.textContent !== label) caption.textContent = label;
    if (Number.isFinite(screen?.[0]) && Number.isFinite(screen?.[1])) {
      caption.style.left = `${screen[0]}px`;
      caption.style.top = `${screen[1]}px`;
      caption.hidden = false;
    } else {
      caption.hidden = true;
    }
    return valid ? endpoint : TRADE_NONE;
  }

  function arm(point, screen) {
    if (!canOrder()) { cancel(); return false; }
    if (!armed) oldCursor = canvas.style.cursor;
    selected = selectionKey();
    armed = true;
    home = TRADE_NONE;
    repaint?.();
    update(point, screen);
    return true;
  }

  function release(point, screen) {
    const target = update(point, screen);
    if (target === TRADE_NONE) return 0;
    if (home === TRADE_NONE) {
      home = target;
      update(point, screen);
      return 0;
    }
    const ordered = issue(home, target);
    if (ordered) cancel();
    else update(point, screen);
    return ordered;
  }

  function decorateButton(button, epoch) {
    const frame = button.ownerDocument.createElement('span');
    frame.className = 'trade-route-icon';
    frame.setAttribute('aria-hidden', 'true');
    if (!art) frame.textContent = '⇄';
    button.setAttribute('aria-keyshortcuts', tradeRouteText.shortcut);
    let hovered = false, pressed = false;
    function paint() {
      button.setAttribute('aria-pressed', String(armed));
      if (art) frame.style.backgroundImage = art.button(nativeMetaEpoch(epoch), pressed || armed ? 'pressed' : hovered ? 'hover' : 'idle');
    }
    button.addEventListener('pointerenter', () => { hovered = true; paint(); });
    button.addEventListener('pointerleave', () => { hovered = pressed = false; paint(); });
    button.addEventListener('pointerdown', event => { if (!event.button) { pressed = true; paint(); } });
    button.addEventListener('pointerup', () => { pressed = false; paint(); });
    button.addEventListener('pointercancel', () => { pressed = false; paint(); });
    button.append(frame);
    repaint = paint;
    paint();
  }

  return Object.freeze({active: () => armed, validate, cancel, arm, update, release, decorateButton});
}
