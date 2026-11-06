import {nativeMetaEpoch} from './original-hud-layout.js';

const directory = 'local-original/trade-wait/';
const manifestHash = '52e8f74a0d5d64c8af3288d8db928a9489b530d220b0fc52a06040ac3d8259c2';
const fallback = {title: 'Wartet auf Handel', description: 'Die Handelseinheit wartet am Markt oder Hafen auf ihren Ladeplatz.'};
const none = 0xffffffff;

async function verifiedBytes(fetchFile, url, expected) {
  const response = await fetchFile(url);
  if (!response.ok) throw Error('Trade status media unavailable');
  const bytes = await response.arrayBuffer();
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  const actual = Array.from(digest, byte => byte.toString(16).padStart(2, '0')).join('');
  if (actual !== expected) throw Error('Trade status media checksum differs');
  return bytes;
}

export async function loadTradeWaitArt({enabled, fetchFile = globalThis.fetch, base = new URL('./', import.meta.url),
  objectURL = URL.createObjectURL.bind(URL)} = {}) {
  if (!enabled) return null;
  try {
    const bytes = await verifiedBytes(fetchFile, new URL(directory + 'manifest.json', base), manifestHash);
    const manifest = JSON.parse(new TextDecoder().decode(bytes)), profile = manifest.profiles.packed7;
    if (manifest.format !== 'ee2-native-trade-wait-art-v1' || profile.scale !== 4 || manifest.effect.SuppressGlow !== 'true') return null;
    async function image(record) {
      if (!record.url.startsWith(directory + 'packed7/') || !/^[\w./-]+\.png$/.test(record.url) || record.url.includes('..')) throw Error('Invalid trade status media path');
      const url = new URL(record.url, base);
      url.searchParams.set('v', record.sha256.slice(0, 16));
      return new Blob([await verifiedBytes(fetchFile, url, record.sha256)], {type:'image/png'});
    }
    const [icon, ...backgrounds] = await Promise.all([image(profile.icon), ...[1, 2, 3].map(theme => image(profile.backgrounds[theme]))]);
    return Object.freeze({icon: objectURL(icon), background: backgrounds.map(objectURL),
      title: manifest.labels.title.text, description: manifest.labels.description.text});
  } catch {
    return null;
  }
}

export function selectedTradeWait(wasm, selection) {
  if (selection?.n !== 1 || typeof wasm?.trade_wait_active !== 'function' || typeof wasm.selected_actor_handle !== 'function') return false;
  const handle = wasm.selected_actor_handle() >>> 0;
  return handle !== none && wasm.trade_wait_active(handle) === 1;
}

export function createTradeWaitStatus(tray, art) {
  const status = art || fallback;
  const icon = art ? tray.ownerDocument.createElement('img') : tray.ownerDocument.createElement('span');
  if (art) {
    icon.src = art.icon;
    icon.alt = status.title;
    icon.width = icon.height = 32;
  } else {
    icon.textContent = status.title;
  }
  tray.title = status.description;
  tray.append(icon);
  let theme;

  function update(visible, epoch = 0) {
    if (tray.hidden === visible) {
      tray.hidden = !visible;
      tray.parentElement.classList.toggle('has-trade-status', visible);
    }
    if (visible && art) {
      const skin = nativeMetaEpoch(epoch);
      if (theme !== skin) {
        theme = skin;
        tray.style.backgroundImage = `url("${art.background[theme - 1]}")`;
      }
    }
  }

  return Object.freeze({update});
}
