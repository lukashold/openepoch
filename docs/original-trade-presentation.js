// Optional media extracted from the user's installed game. Game definitions
// own the IDs and availability. This catalog only projects source families.
const CONTROL = 'local-original/trade-route/manifest.json';
const CONTROL_SHA = 'a8d3567c9abe47a4a398916c86ea2d50075e34c346a79ef01b838c90fe3e1079';
const TRADE = {folder:'trade-presentation',sha:'db08f20a02cbd1fd1f3e559ac15df1b5e1e58c596cf57c2f8543521ddc22f341',
  format:'ee2-native-trade-presentation-v1',ids:{TradeCart:81,MerchantShip:82},starts:{TradeCart:[1,5,10,15],MerchantShip:[0,5,10,15]}};
const FIGHTER_BOMBER = {folder:'fighter-bomber',sha:'b1e5280f590a6b96871b13628be58387fc507c43882c7817d5b871127900cd86',
  format:'ee2-native-fighter-bomber-presentation-v1',ids:{FighterBomber:83},starts:{FighterBomber:[10,11,12,13,14,15]}};
const relative = (path, prefix) => typeof path === 'string' && path.startsWith(prefix) && /^[\w./-]+$/.test(path) && !path.split('/').includes('..');
const hash = value => /^[a-f0-9]{64}$/.test(value || '');

async function verified(fetchFile, url, sha256) {
  if (!hash(sha256)) throw Error('Missing trade asset hash');
  const response = await fetchFile(url);
  if (!response.ok) throw Error(`Trade asset unavailable (${response.status})`);
  const bytes = await response.arrayBuffer();
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  if (Array.from(digest, value => value.toString(16).padStart(2, '0')).join('') !== sha256) throw Error('Trade asset hash differs');
  return bytes;
}

async function loadPresentation(spec,{enabled, fetchFile = globalThis.fetch, bitmap = globalThis.createImageBitmap,
  base = new URL('./', import.meta.url)} = {}) {
  if (!enabled) return null;
  const root=`local-original/${spec.folder}/`;
  const decode = async (url, sha) => JSON.parse(new TextDecoder().decode(await verified(fetchFile, new URL(url, base), sha)));
  try {
    const catalog = await decode(root+'manifest.json',spec.sha);
    const count=Object.values(spec.starts).reduce((total,epochs)=>total+epochs.length-1,0);
    if (catalog.format !== spec.format || catalog.profile !== 'up15-csv5' || catalog.rows?.length !== count) throw Error('Unexpected original presentation catalog');
    const rows = new Map(), seen = new Set();
    for (const row of catalog.rows) {
      const id = spec.ids[row.family], interval = spec.starts[row.family], index = interval?.indexOf(row.epoch);
      const icon = row.icons?.packed7;
      if (!id || index < 0 || interval[index + 1] !== row.until || seen.has(`${id}:${row.epoch}`) || row.model.sourceFamily !== row.family ||
          !relative(row.model.url, root+'models/') || !hash(row.model.sha256) ||
          (row.model.lodFarUrl && (!relative(row.model.lodFarUrl, root+'models/') || !hash(row.model.lodFarSha256))) ||
          !relative(icon?.url, root+'icons/packed7/') || !hash(icon.sha256) || icon.width !== 308 || icon.height !== 240) throw Error('Invalid original presentation row');
      seen.add(`${id}:${row.epoch}`);
      const entries = rows.get(id) || [];
      entries.push(row);
      rows.set(id, entries);
    }
    const modelRows = catalog.rows.map(row => ({...row.model, id: spec.ids[row.family], regionalFallback: true}));
    const lookup = (id, epoch) => rows.get(id)?.find(row => epoch >= row.epoch && epoch < row.until);
    const bitmaps = new Map();
    const [teamManifest, sailManifest, audioManifest] = await Promise.all([
      decode(catalog.teamManifest, catalog.teamManifestSha256).catch(() => null),
      catalog.sailManifest?decode(catalog.sailManifest, catalog.sailManifestSha256).catch(() => null):null,
      decode(catalog.audioManifest, catalog.audioManifestSha256).catch(() => null),
      ...catalog.rows.map(async row => {
        const icon = row.icons.packed7;
        try {
          const image = await bitmap(new Blob([await verified(fetchFile, new URL(icon.url, base), icon.sha256)]));
          if (image.width === icon.width && image.height === icon.height) bitmaps.set(icon.url, image);
          else image.close?.();
        } catch {/* Existing procedural icons remain available. */}
      }),
    ]);
    return Object.freeze({modelRows, teamManifest, sailManifest, audioManifest,
      icon: (id, epoch) => bitmaps.get(lookup(id, epoch)?.icons.packed7.url),
      name(id, epoch, count) {
        const labels = lookup(id, epoch)?.labels;
        return (count > 1 ? labels?.displayNamePlural : labels?.displayName)?.text;
      },
      description: (id, epoch) => lookup(id, epoch)?.labels.VerboseTooltip?.text});
  } catch {
    return null;
  }
}

export const loadTradePresentation = options => loadPresentation(TRADE,options);
export const loadFighterBomberPresentation = options => loadPresentation(FIGHTER_BOMBER,options);

export async function loadTradeRouteArt({enabled, fetchFile = globalThis.fetch, base = new URL('./', import.meta.url)} = {}) {
  if (!enabled) return null;
  try {
    const art = JSON.parse(new TextDecoder().decode(await verified(fetchFile, new URL(CONTROL, base), CONTROL_SHA)));
    if (art.format !== 'ee2-native-trade-route-art-v1') return null;
    const urls = new Map();
    const record = asset => {
      if (!relative(asset?.url, 'local-original/trade-route/') || !hash(asset.sha256)) throw Error('Invalid trade route sprite');
      const url = new URL(asset.url, base);
      url.searchParams.set('v', asset.sha256.slice(0, 16));
      urls.set(asset, `url("${url.href}")`);
    };
    for (const theme of [1, 2, 3]) for (const state of ['idle', 'hover', 'pressed']) record(art.buttons[theme][state]);
    record(art.cursors.valid); record(art.cursors.invalid);
    return Object.freeze({button: (theme, state) => urls.get(art.buttons[theme][state]),
      cursor: valid => `${urls.get(valid ? art.cursors.valid : art.cursors.invalid)} 16 16, ${valid ? 'crosshair' : 'not-allowed'}`});
  } catch {
    return null;
  }
}
