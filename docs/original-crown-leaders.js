// Optional presentation from a separately imported local media folder.
// Definitions and game commands stay authoritative in the engine.
const root = 'local-original/crown-leaders/';
const modelCatalog = {url:root+'models.json',sha256:'12495789959205b29a3dc97f01176c1bbd1d22989d59a5ba28b0055aea78887e'};
const uiCatalog = {url:root+'ui.json',sha256:'5ec2d9e24645e3bc09154a94329bb9bfb6fa449b5237f43234c58a32a4b5bc37'};
const families = new Map([[87,'LeaderMilitary'],[88,'LeaderEconomic'],[89,'LeaderImperial']]);
const eraStarts = [0,5,10];
const hash = /^[a-f0-9]{64}$/;

function localAsset(row, prefix='local-original/') {
  return typeof row?.url === 'string' && row.url.startsWith(prefix) && /^[\w./-]+$/.test(row.url) &&
    !row.url.split('/').some(part => part === '.' || part === '..') && hash.test(row.sha256);
}

function taggedURL(row, base) {
  const url = new URL(row.url, base);
  url.searchParams.set('v', row.sha256.slice(0,16));
  return url;
}

async function verify(fetchFile, row, base) {
  if (!localAsset(row)) throw Error('Invalid local Crown leader asset');
  const response = await fetchFile(taggedURL(row,base));
  if (!response.ok) throw Error('Crown leader asset unavailable');
  const bytes = await response.arrayBuffer();
  if (row.bytes !== undefined && row.bytes !== bytes.byteLength) throw Error('Crown leader asset length differs');
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
  if (Array.from(digest,value=>value.toString(16).padStart(2,'0')).join('') !== row.sha256) {
    throw Error('Crown leader asset hash differs');
  }
  return bytes;
}

function sourceRows(catalog) {
  if (catalog.format !== 'ee2-native-regional-model-manifest-v1' || catalog.models?.length !== 36) {
    throw Error('Invalid Crown leader model catalog');
  }
  const index = new Map();
  for (const row of catalog.models) {
    const family = families.get(row.id), era = row.epoch;
    if (!family || row.assetKind !== 0 || !eraStarts.includes(era) || row.until !== era+5 ||
        row.sourceFamily !== family || row.sourceVisual !== family+(era ? '_'+(era+1) : '') ||
        !localAsset(row,root+'models/') || !localAsset(row.animationPayload,root+'animations/') ||
        !localAsset(row.sharedBody,'local-original/king-regicide/models/') || !row.nativeScaleBinding?.verified ||
        row.nativeScaleBinding.forceScale !== .3 || row.nativeScaleBinding.sourceScale !== .2121320366859436 ||
        row.nativeScaleBinding.sourceSha256 !== row.source?.sha256 || !Array.isArray(row.regionalCivilizations) ||
        !row.regionalCivilizations.length) throw Error('Invalid Crown leader model binding');
    for (const civ of row.regionalCivilizations) {
      const key = `${row.id}:${era}:${civ}`;
      if (!Number.isInteger(civ) || civ < 0 || civ >= 14 || index.has(key)) throw Error('Invalid Crown leader region');
      index.set(key,row);
    }
  }
  if (index.size !== 3*3*14) throw Error('Incomplete Crown leader regions');
  return index;
}

export async function loadCrownLeaderPresentation({enabled,fetchFile=globalThis.fetch,bitmap=globalThis.createImageBitmap,
  base=new URL('./',import.meta.url)}={}) {
  if (!enabled) return null;
  const decode = async row => JSON.parse(new TextDecoder().decode(await verify(fetchFile,row,base)));
  try {
    const [model,ui] = await Promise.all([decode(modelCatalog),decode(uiCatalog)]);
    const index = sourceRows(model);
    if (ui.format !== 'ee2-native-crown-leader-ui-v1' || !localAsset(ui.audioManifest,'local-original/audio/')) return null;
    const team = {url:model.teamManifest,sha256:model.teamManifestSha256};
    const assets = new Map();
    for (const profile of Object.values(ui.profiles)) {
      for (const rows of Object.values(profile.icons)) {
        for (const row of rows) {
          if (!localAsset(row,'local-original/kingregicide-icons/')) throw Error('Invalid Crown leader icon');
          assets.set(row.url,row);
        }
      }
    }
    const bitmaps = new Map();
    const [teamManifest,audioManifest] = await Promise.all([
      decode(team), decode(ui.audioManifest).catch(()=>null),
      ...[...assets.values()].map(async row=>{
        try {
          const image = await bitmap(new Blob([await verify(fetchFile,row,base)]));
          if (image.width===row.width && image.height===row.height) bitmaps.set(row.url,image);
          else image.close?.();
        } catch {/* The ordinary icon fallback remains available. */}
      }),
    ]);
    const lookup = (id,epoch,civ=0) => {
      if (!Number.isInteger(epoch) || epoch < 0 || epoch >= 15) return undefined;
      if (!Number.isInteger(civ) || civ < 0 || civ >= 14) civ=0;
      return index.get(`${id}:${Math.floor(epoch/5)*5}:${civ}`);
    };
    const visual = (id,epoch) => lookup(id,epoch)?.sourceVisual;
    return Object.freeze({modelRows:model.models,audioModelRows:model.models.filter(row=>row.regionalFallback),
      teamManifest,audioManifest,lookup,
      icon(id,epoch,civ=0,packed=7) {
        const row=lookup(id,epoch,civ);
        const icon=ui.profiles['packed'+packed]?.icons[row?.sourceVisual]?.find(asset=>
          asset.civilizations.some(candidate=>row.regionalCivilizations.includes(candidate)));
        return bitmaps.get(icon?.url);
      },
      name(id,epoch,count=1) {
        const text=ui.unitLabels[visual(id,epoch)];
        return (count>1 ? text?.plural : text?.name)?.text;
      },
      description:(id,epoch)=>ui.unitLabels[visual(id,epoch)]?.help.text,
      power(id,epoch,theme,packed=7) {
        const name=ui.activePowers[visual(id,epoch)], action=ui.actions[name];
        if (!action) return null;
        const states=ui.profiles['packed'+packed]?.powers[name]?.[String(theme)];
        const art=states && Object.fromEntries(Object.entries(states).map(([key,row])=>
          [key,localAsset(row) ? `url("${taggedURL(row,base).href}")` : null]));
        return {name,label:action.label.text,help:action.help.text,art:art??null,hotkey:ui.genericHotkey.key};
      },
    });
  } catch {
    return null;
  }
}
