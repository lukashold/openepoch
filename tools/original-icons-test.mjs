import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {loadOriginalEpochIcons,originalIcon} from '../docs/original-assets.js';

const manifest=JSON.parse(await readFile(new URL('../docs/local-original/models/manifest.json',import.meta.url)));
const originalFetch=globalThis.fetch,originalDecode=globalThis.createImageBitmap;
const loads=[];
try{
  globalThis.fetch=async url=>{loads.push(url.href);return {ok:true,blob:async()=>({url:url.href})};};
  globalThis.createImageBitmap=async blob=>({source:blob.url,width:64,height:64});
  const base={source:'base'},assets={enabled:true,icons:new Map([[9,base]]),iconVariants:new Map(),errors:[]};
  // Reverse the source list to make the loader's asynchronous completion order
  // irrelevant to the first/last native transition shown in the HUD.
  await loadOriginalEpochIcons(assets,{epochIconBindings:[...manifest.epochIconBindings].reverse()});
  assert.equal(assets.errors.length,0);
  assert.equal(loads.length,new Set(manifest.epochIconBindings.map(row=>row.url)).size,'shared source PNG fetched once');
  assert.equal(originalIcon(assets,9,0),base,'Stone-Age base icon retained before native transition');
  assert.match(originalIcon(assets,9,2).source,/swordsman_we-sprite\.png$/,'Bronze-Age swordsman icon');
  assert.match(originalIcon(assets,9,6).source,/musketeer_we-sprite\.png$/,'Renaissance musket icon');
  assert.match(originalIcon(assets,9,10).source,/rifleman_we-sprite\.png$/,'Modern-Age rifle icon');
  assert.match(originalIcon(assets,9,12).source,/assaultrifleman_we-sprite\.png$/,'Digital-Age assault-rifle icon');
  assert.equal(originalIcon(assets,9,14),originalIcon(assets,9,12),'last visual held consistently with renderer in a missing-model interval');
  assert.equal(originalIcon(assets,999,0),undefined,'unmapped ID remains on normal HUD fallback');
  const atmospheric={source:'original atmospheric base icon'},complete={enabled:true,icons:new Map([[44,atmospheric],[9,base]]),iconVariants:new Map(),errors:[]};
  await loadOriginalEpochIcons(complete,{...manifest,epochIconBindings:manifest.epochIconBindings.filter(row=>row.id!==37||row.epoch!==14)});
  assert.match(originalIcon(complete,37,12).source,/jetfighter-sprite\.png$/);
  assert.equal(originalIcon(complete,37,14),atmospheric,'a new Atmospheric model without a proven native SpriteName never keeps the older Jet icon');
  const modelFallback=complete.epochIcons.get(37).find(row=>row.epoch===14);
  assert.equal(modelFallback.nativeExact,false);assert.equal(modelFallback.iconFallback,'associated-model-base-icon');
  const house13={},temple4={},temple13={},westHouse13={};
  const buildings={icons:new Map(),iconVariants:new Map([['icon_unit_house_13',house13],['icon_unit_temple_4',temple4],['icon_unit_temple_13',temple13]])};
  assert.equal(originalIcon(buildings,1,12),house13);assert.equal(originalIcon(buildings,51,3),temple4);assert.equal(originalIcon(buildings,51,12),temple13);
  buildings.iconVariants.set('icon_unit_house_13_we',westHouse13);assert.equal(originalIcon(buildings,1,12),westHouse13,'verified Western variant preferred at the same epoch');
  const disabled={enabled:false};await loadOriginalEpochIcons(disabled,manifest);assert.equal(disabled.epochIcons.size,0);
  globalThis.fetch=async()=>({ok:false,status:404});
  const missing={enabled:true,icons:new Map([[9,base]]),iconVariants:new Map(),errors:[]};
  await loadOriginalEpochIcons(missing,{epochIconBindings:[manifest.epochIconBindings[0]]});
  assert.equal(missing.errors.length,1);assert.equal(originalIcon(missing,9,2),base,'missing optional PNG preserves base icon');
}finally{globalThis.fetch=originalFetch;globalThis.createImageBitmap=originalDecode;}
console.log('PASS original epoch icons: exact Bronze/Renaissance/Modern/Digital native transitions, asynchronous ordering, source-image deduplication, held-model consistency and optional-asset fallbacks');
