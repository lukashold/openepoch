import {activeValleyGeometry} from './authored-view.js';
import {loadAuthoredValleyTerrain} from './authored-terrain.js';
import {archiveForestBindings, archiveForestFrame} from './archive-forest.js';
import {bindValleyWorld0,valleyWorld0Frame} from './archive-world0.js';
import {loadOriginalModels} from './original-renderer.js?v=native36';

const path = 'local-original/valley-preview/manifest.json';
const digest = '75dc890404bf8f50f69975ae796d38d608e03f1a7e85e61405083615eb82419c';
const authoredTest = {url:'local-original/valley-owned-qa/seed-2.vow1',sha256:'68ca55d581c3e855a17aad58461575f9a160a9c6bd8fa90e081cbecee05bfefc'};
const magic = [0x56, 0x33, 0x37, 0x53]; // V37S
const hex = bytes => Array.from(bytes, n => n.toString(16).padStart(2, '0')).join('');

export function valleySaveLocator(bytes) {
  if (!(bytes instanceof Uint8Array) || !magic.every((n, i) => bytes[i] === n)) return null;
  return {map: hex(bytes.subarray(12, 44)), fixture: hex(bytes.subarray(44, 76))};
}

async function readLocal(path) {
  const response = await fetch(new URL(path, import.meta.url), {cache: 'no-store'});
  if (!response.ok) throw new Error('Lokale Valley-Datei fehlt.');
  return new Uint8Array(await response.arrayBuffer());
}

async function checkedAsset(read, row, supplied = null) {
  if (!/^local-original\/[a-z0-9_.\/-]+$/.test(row.url) || row.url.includes('..') || !/^[0-9a-f]{64}$/.test(row.sha256)) {
    throw new Error('Ungültiger lokaler Valley-Pfad.');
  }
  const bytes = supplied ?? await read(row.url);
  if (hex(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))) !== row.sha256) {
    throw new Error('Lokale Valley-Datei stimmt nicht mit ihrer Prüfsumme überein.');
  }
  return bytes;
}

export async function installedValleyManifest() {
  if (!globalThis.crypto?.subtle) return null;
  try { return await checkedAsset(readLocal, {url: path, sha256: digest}); }
  catch { return null; }
}

function supply(wasm, bytes, allocate) {
  const ptr = allocate(bytes.length);
  if (!ptr) return false;
  new Uint8Array(wasm.memory.buffer, ptr, bytes.length).set(bytes);
  return true;
}

export async function loadValleyPreview({requested, authoredRequested, neutralRequested = false, requireAuthored = false,
  manifest = null, wasm, gl, teams, read = readLocal, onUnavailable = () => {}}) {
  if (!requested || typeof wasm.valley39_commit !== 'function' || !globalThis.crypto?.subtle) return null;
  const checked = row => checkedAsset(read, row);
  const json = bytes => JSON.parse(new TextDecoder().decode(bytes));
  try {
    const index = json(await checkedAsset(read, {url: path, sha256: digest}, manifest));
    if (index.format !== 'ee2-private-valley-preview-v1' || index.localOnly !== true ||
        index.start.join(',') !== '0,0' || index.fixtures.length !== 4) throw new Error('Ungültige Valley-Vorschau.');
    const [map, terrainManifest, forestManifest, world0Manifest, ...packets] = await Promise.all(
      [index.map, index.terrain, index.forest, index.world0, ...index.fixtures].map(checked));
    const neutralCommit = requireAuthored ? wasm.valley75_commit : wasm.valley71_commit;
    const neutralSupported = !neutralRequested || typeof neutralCommit === 'function';
    const authored = authoredRequested && neutralSupported ? await checked(authoredTest).catch(() => null) : null;
    if (requireAuthored && !authored) return null;
    const stageMap = () => supply(wasm, map, wasm.valley37_alloc) && wasm.valley37_prepare() === 1;
    if (!stageMap()) throw new Error('Valley-Karte konnte nicht vorbereitet werden.');
    const geometry = activeValleyGeometry(wasm, true);
    if (geometry.sourceArchiveSha256 !== index.sourceArchiveSha256) throw new Error('Valley-Quellkarte stimmt nicht überein.');
    const terrain = json(terrainManifest), forest = json(forestManifest), world0 = json(world0Manifest);
    const assetMapping = archiveForestBindings(forest, index.sourceArchiveSha256);
    const staticBindings = bindValleyWorld0(world0, index.sourceArchiveSha256, index.fixtures, packets);
    await Promise.all([...forest.models, ...forest.textures.map(row => ({url: row.url, sha256: row.png_sha256})),
      ...world0.models, ...world0.textures].map(checked));
    const terrainRows = new Map(terrain.bindings.map(row => [row.url, row]));
    const readTexture = async url => {
      const bytes = await checked(terrainRows.get(url));
      const bitmap = await createImageBitmap(new Blob([bytes], {type: 'image/png'}), {premultiplyAlpha: 'none', colorSpaceConversion: 'none'});
      return {bytes, bitmap};
    };
    const ground = await loadAuthoredValleyTerrain(gl, geometry, terrain, readTexture);
    const trees = await loadOriginalModels(gl, true, teams, undefined,
      {assetMapping, manifestURL: index.forest.url, initialSails: false, nativeLighting: false});
    if (trees.errors.length || trees.models.size !== forest.models.length) throw new Error('Valley-Waldgrafiken konnten nicht geladen werden.');
    const statics = await loadOriginalModels(gl, true, teams, undefined,
      {assetMapping: staticBindings.assetMapping, manifestURL: index.world0.url, initialSails: false, nativeLighting: false});
    if (statics.errors.length || statics.models.size !== world0.models.length) throw new Error('Valley-Objekte konnten nicht geladen werden.');
    const defaultFixture = index.fixtures.findIndex(row => row.site === 0 && row.checkpoint === 0);
    const stageFixture = bytes => !!bytes && stageMap() && supply(wasm, bytes, wasm.valley37_fixture_alloc);
    return Object.freeze({
      authoredAvailable: !!authored,
      start(visibility, useAuthored = false) {
        if (!Number.isInteger(visibility) || visibility < 0 || visibility > 2 ||
            !stageFixture(useAuthored ? authored : packets[defaultFixture])) return false;
        const commit = useAuthored && neutralRequested ? neutralCommit : wasm.valley39_commit;
        return wasm.valley37_prepare_fixture() === 1 && commit(visibility) === 1;
      },
      prepareLoad(locator) {
        if (!locator || locator.map !== index.map.sha256) return false;
        return stageFixture(locator.fixture === authoredTest.sha256 ? authored :
          packets[index.fixtures.findIndex(row => row.sha256 === locator.fixture)]);
      },
      drawTerrain(vp, fog) { ground.draw(vp, fog); },
      drawWorld(view, bounds) {
        const fog = new Uint8Array(wasm.memory.buffer, wasm.fog_ptr(), geometry.backing ** 2);
        const options = {vp: view.vp, eye: view.eye, zoom: view.zoom, epochs: [0, 0], capture: true};
        trees.draw({...archiveForestFrame(forest, fog, bounds), ...options});
        const currentCase = wasm.valley37_view_info(3);
        statics.draw({...valleyWorld0Frame(staticBindings, currentCase, fog, bounds, wasm.valley37_view_info(4) === 1), ...options});
      },
    });
  } catch (error) {
    onUnavailable(error);
    return null;
  }
}
