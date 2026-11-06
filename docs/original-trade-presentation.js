// Optional media extracted from the user's installed game. Game definitions
// own the IDs and availability. This catalog only projects source families.
const CATALOG = 'local-original/trade-presentation/manifest.json';
const CATALOG_SHA = 'db08f20a02cbd1fd1f3e559ac15df1b5e1e58c596cf57c2f8543521ddc22f341';
const CONTROL = 'local-original/trade-route/manifest.json';
const CONTROL_SHA = 'a8d3567c9abe47a4a398916c86ea2d50075e34c346a79ef01b838c90fe3e1079';
const sourceIds = {TradeCart: 81, MerchantShip: 82};
const starts = {TradeCart: [1, 5, 10, 15], MerchantShip: [0, 5, 10, 15]};
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

export async function loadTradePresentation({enabled, fetchFile = globalThis.fetch, bitmap = globalThis.createImageBitmap,
  base = new URL('./', import.meta.url)} = {}) {
  if (!enabled) return null;
  const decode = async (url, sha) => JSON.parse(new TextDecoder().decode(await verified(fetchFile, new URL(url, base), sha)));
  try {
    const catalog = await decode(CATALOG, CATALOG_SHA);
    if (catalog.format !== 'ee2-native-trade-presentation-v1' || catalog.profile !== 'up15-csv5' || catalog.rows?.length !== 6) throw Error('Unexpected trade presentation catalog');
    const rows = new Map(), seen = new Set();
    for (const row of catalog.rows) {
      const id = sourceIds[row.family], interval = starts[row.family], index = interval?.indexOf(row.epoch);
      const icon = row.icons?.packed7;
      if (!id || index < 0 || interval[index + 1] !== row.until || seen.has(`${id}:${row.epoch}`) || row.model.sourceFamily !== row.family ||
          !relative(row.model.url, 'local-original/trade-presentation/models/') || !hash(row.model.sha256) ||
          (row.model.lodFarUrl && (!relative(row.model.lodFarUrl, 'local-original/trade-presentation/models/') || !hash(row.model.lodFarSha256))) ||
          !relative(icon?.url, 'local-original/trade-presentation/icons/packed7/') || !hash(icon.sha256) || icon.width !== 308 || icon.height !== 240) throw Error('Invalid trade presentation row');
      seen.add(`${id}:${row.epoch}`);
      const entries = rows.get(id) || [];
      entries.push(row);
      rows.set(id, entries);
    }
    const modelRows = catalog.rows.map(row => ({...row.model, id: sourceIds[row.family], regionalFallback: true}));
    const lookup = (id, epoch) => rows.get(id)?.find(row => epoch >= row.epoch && epoch < row.until);
    const bitmaps = new Map();
    const [teamManifest, sailManifest, audioManifest] = await Promise.all([
      decode(catalog.teamManifest, catalog.teamManifestSha256).catch(() => null),
      decode(catalog.sailManifest, catalog.sailManifestSha256).catch(() => null),
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
