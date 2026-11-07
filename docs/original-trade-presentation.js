// Optional media extracted from the user's installed game. Game definitions
// own the IDs and availability. This catalog only projects source families.
const CONTROL = 'local-original/trade-route/manifest.json';
const CONTROL_SHA = 'a8d3567c9abe47a4a398916c86ea2d50075e34c346a79ef01b838c90fe3e1079';
const TRADE = {folder:'trade-presentation',sha:'db08f20a02cbd1fd1f3e559ac15df1b5e1e58c596cf57c2f8543521ddc22f341',
  format:'ee2-native-trade-presentation-v1',ids:{TradeCart:81,MerchantShip:82},starts:{TradeCart:[1,5,10,15],MerchantShip:[0,5,10,15]}};
const FIGHTER_BOMBER = {folder:'fighter-bomber',sha:'b1e5280f590a6b96871b13628be58387fc507c43882c7817d5b871127900cd86',
  format:'ee2-native-fighter-bomber-presentation-v1',ids:{FighterBomber:83},starts:{FighterBomber:[10,11,12,13,14,15]}};
const CARRIER = {folder:'aircraft-carrier',sha:'714d9458f53419acf3c1dfbcee5d67999450e6645161dab98f66b5e5a8c8d6f8',
  format:'ee2-native-aircraft-carrier-presentation-v1',family:'AircraftCarrier',ids:{AircraftCarrier:84},starts:{AircraftCarrier:[10,13,15]}};
const PRIEST = {folder:'priest',sha:'1289d04a019f5637f20937d2149101d353f4168ac302fe16b2a0e0b3e68a2543',
  format:'ee2-native-priest-presentation-v1',family:'Priest',ids:{Priest:85},starts:{Priest:[0,3,6,9,12,15]}};
const SPY = {folder:'spy',sha:'9a284d54367bbb4cbee2b0f49582f1c5631897ea9574dea5df34aef8453726ac',
  format:'ee2-native-spy-presentation-v1',family:'Spy',ids:{Spy:86},starts:{Spy:[0,3,6,9,12,15]}};
const carrierLoop = {url:'local-original/audio/aircraft-carrier/fx_move_boat_4_15-pcm.wav',sha256:'442d1a24b43abe1e05102ad85022ef3eecb32bcdd56393618deaacc118b90be9',bytes:170710};
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
    const sourceMatches = spec.family ? catalog.family === spec.family : catalog.profile === 'up15-csv5';
    if (catalog.format !== spec.format || !sourceMatches || catalog.rows?.length !== count) throw Error('Unexpected original presentation catalog');
    const rows = new Map(), seen = new Set(), regional = spec === PRIEST || spec === SPY;
    const modelRows = [];
    for (const row of catalog.rows) {
      const family = spec.family ?? row.family;
      const id = spec.ids[family], interval = spec.starts[family], index = interval?.indexOf(row.epoch);
      const variants = regional ? row.models : [row.model];
      const icons = regional ? row.regionalIcons?.packed7 : [row.icons?.packed7];
      const validCivs = entries => {
        const civs = entries.flatMap(entry => entry);
        return civs.length === 14 && Array.from({length:14},(_,civ)=>civ).every(civ=>civs.includes(civ));
      };
      if (!id || index < 0 || interval[index + 1] !== row.until || seen.has(`${id}:${row.epoch}`) ||
          !Array.isArray(variants) || !variants.length || !Array.isArray(icons) || !icons.length ||
          variants.some(model => model?.sourceFamily !== family ||
            !relative(model.url, root+'models/') || !hash(model.sha256) ||
            (model.lodFarUrl && (!relative(model.lodFarUrl, root+'models/') || !hash(model.lodFarSha256)))) ||
          icons.some(icon => !relative(icon?.url, root+'icons/packed7/') || !hash(icon.sha256) ||
            icon.width !== 308 || icon.height !== 240) ||
          (regional && (!validCivs(variants.map(model=>model.regionalCivilizations)) ||
            !validCivs(icons.map(icon=>icon.civilizations))))) throw Error('Invalid original presentation row');
      seen.add(`${id}:${row.epoch}`);
      const entries = rows.get(id) || [];
      entries.push(row);
      rows.set(id, entries);
      for (const model of variants) modelRows.push({
        ...model, id, regionalFallback: !regional || model.regionalCivilizations.includes(0),
        ...(spec===SPY?{spyClips:{...model.spyClips,2:'sabotage',3:'gatherintel',4:'gatherintel',5:'sabotage'}}:{}),
      });
    }
    const lookup = (id, epoch) => rows.get(id)?.find(row => epoch >= row.epoch && epoch < row.until);
    const bitmaps = new Map();
    const [teamManifest, sailManifest, audioManifest] = await Promise.all([
      decode(catalog.teamManifest, catalog.teamManifestSha256).catch(() => null),
      catalog.sailManifest?decode(catalog.sailManifest, catalog.sailManifestSha256).catch(() => null):null,
      decode(catalog.audioManifest, catalog.audioManifestSha256).catch(() => null),
      ...[...new Map(catalog.rows.flatMap(row => regional ? row.regionalIcons.packed7 : [row.icons.packed7])
        .map(icon => [icon.url,icon])).values()].map(async icon => {
        try {
          const image = await bitmap(new Blob([await verified(fetchFile, new URL(icon.url, base), icon.sha256)]));
          if (image.width === icon.width && image.height === icon.height) bitmaps.set(icon.url, image);
          else image.close?.();
        } catch {/* Existing procedural icons remain available. */}
      }),
    ]);
    return Object.freeze({modelRows, audioModelRows:modelRows.filter(row=>row.regionalFallback), teamManifest, sailManifest, audioManifest,
      movementAudio: spec === CARRIER && audioManifest ? carrierMovementAudio(audioManifest, lookup, base) : null,
      garrisonFlag: spec === CARRIER ? {url:catalog.garrisonFlag,sha256:catalog.garrisonFlagSha256,carrierAt:epoch=>lookup(84,epoch)} : null,
      icon(id, epoch, civilization=0) {
        const row=lookup(id,epoch);
        const icon=regional ? row?.regionalIcons.packed7.find(icon=>icon.civilizations.includes(civilization)) : row?.icons.packed7;
        return bitmaps.get(icon?.url);
      },
      name(id, epoch, count) {
        const labels = lookup(id, epoch)?.labels;
        return (count > 1 ? labels?.displayNamePlural : labels?.displayName)?.text;
      },
      description: (id, epoch) => lookup(id, epoch)?.labels.VerboseTooltip?.text});
  } catch {
    return null;
  }
}

function carrierMovementAudio(audio, lookup, base) {
  const name = 'fx_move_boat_4';
  const sound = audio.events?.[name];
  if (!sound || !audio.assets?.[sound.file]) return null;
  const event = {...carrierLoop, url:new URL(carrierLoop.url, base), gain:sound.volume,
    pitchVariation:sound.pitchVariation, maxInstances:sound.maxInstances, sampleRate:22050};
  const row = (id, epoch) => {
    const source = lookup(id, epoch);
    if (!source) return undefined;
    // The cue-only Walk state contains a persistent start. Re-scanning its
    // zero key has no effect until the next accepted animation change.
    const keys = source.states.walk.textKeys.filter(key => key.event === name);
    return {states:{walk:{keys, clipDuration:3.4028234663852886e38}}};
  };
  return {events:new Map([[name, event]]), row};
}

export const loadTradePresentation = options => loadPresentation(TRADE,options);
export const loadFighterBomberPresentation = options => loadPresentation(FIGHTER_BOMBER,options);
export const loadCarrierPresentation = options => loadPresentation(CARRIER,options);

export const loadPriestPresentation = options => loadPresentation(PRIEST,options);
export const loadSpyPresentation = options => loadPresentation(SPY,options);

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
