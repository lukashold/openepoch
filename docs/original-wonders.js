import {nativeUnitUndertextureSizing} from './original-ground-native.js';

const PREFIX = 'local-original/wonders/';
const REGIONS = ['Western', 'MiddleEast', 'FarEast', 'MesoAmerican'];
const EPOCHS = [1, 6, 11];
const SOURCE_SCALE = 0.02828427031636238;
const SOURCE_VISUAL_YAW_OFFSET = -1.5707963705062866;
const HASH = /^[a-f0-9]{64}$/;

function asset(row, path) {
  return row?.url === PREFIX + path && HASH.test(row.sha256) && Number.isSafeInteger(row.bytes) && row.bytes > 0;
}

function tagged(row) {
  return `${row.url}?v=${row.sha256.slice(0, 16)}`;
}

function validPair(pair, positive = false) {
  return Array.isArray(pair) && pair.length === 2 && pair.every(value => Number.isFinite(value) && (!positive || value > 0));
}

function typedSprite(sprite, textures) {
  const image = textures.get(sprite?.url);
  if (!image || sprite.sha256 !== image.sha256 || !/^under_\w+$/.test(sprite.sprite) ||
      sprite.imageWidth !== image.width || sprite.imageHeight !== image.height || String(sprite.uv) !== '0,0,1,1') {
    throw new Error('Ungültige Weltwunder-Bodengrafik');
  }
  const sequence = urls => {
    if (!Array.isArray(urls) || ![1, 4, 12].includes(urls.length) || urls.some(url => !textures.has(url))) {
      throw new Error('Ungültige Saisonfolge für Weltwunder');
    }
    return urls.map(url => tagged(textures.get(url)));
  };
  return {...sprite, url: tagged(image), seasonalUrls: sequence(sprite.seasonalUrls), climates: {
    Arid: sequence(sprite.climates?.Arid), Tropical: sequence(sprite.climates?.Tropical),
  }};
}

function manifestRows(bodies, geometry) {
  if (bodies?.format !== 'ee2-native-wonders-v1' || geometry?.format !== 'ee2-native-wonder-geometry-v1' ||
      !HASH.test(geometry.bodyManifestSha256) || geometry.native?.sourceScale !== SOURCE_SCALE ||
      geometry.native.visualYawOffset !== SOURCE_VISUAL_YAW_OFFSET ||
      !Array.isArray(geometry.rows) || geometry.rows.length !== 12) throw new Error('Unbekanntes Weltwunder-Medienformat');
  const textures = new Map();
  for (const [name, row] of Object.entries(geometry.textures ?? {})) {
    if (!/^under_\w+$/.test(name) || !asset(row, `ground/${name}.png`) ||
        ![row.width, row.height].every(value => Number.isInteger(value) && value > 0)) throw new Error('Ungültige Weltwunder-Textur');
    textures.set(row.url, row);
  }
  const modelRows = [], groundRows = [], descriptions = new Map();
  for (const [index, row] of geometry.rows.entries()) {
    const id = 90 + index, region = REGIONS[Math.floor(index / EPOCHS.length)], epoch = EPOCHS[index % EPOCHS.length];
    const family = `Wonder${region}_${String(epoch).padStart(2, '0')}`, decal = 'Under' + family;
    const definition = bodies.families?.[family], model = bodies.models?.[family], binding = row.sourceScaleBinding;
    if (row.id !== id || row.family !== family || row.epoch !== epoch || row.region !== region || row.decal !== decal ||
        definition?.epoch !== epoch || definition.region !== region || definition.children?.[0] !== decal ||
        !asset(model, `models/${family}.json`) || !HASH.test(model.sourceSha256) || binding?.verified !== true ||
        binding.scaleType !== 'kScaleBuilding' || binding.sourceScale !== SOURCE_SCALE || binding.sourceSha256 !== model.sourceSha256 ||
        !row.raw || !validPair(row.raw.size, true) || !validPair(row.raw.footprint, true) ||
        !['centerOffset', 'underSize', 'underOffset'].every(key => validPair(row.raw[key])) ||
        row.deathDelay !== 10 || row.deathFade !== 10) throw new Error(`Ungültige Weltwunder-Bindung ${id}`);

    const shape = nativeUnitUndertextureSizing(row.raw);
    if (String(definition.size) !== String(row.raw.size) || String(definition.footprint) !== String(row.raw.footprint) ||
        String(shape.bodySize) !== String(row.rectangles?.body) || String(shape.footprintSize) !== String(row.rectangles?.footprint) ||
        !['width', 'depth', 'offsetX', 'offsetY', 'mode'].every(key => shape[key] === row.rectangles.underQuad[key])) {
      throw new Error(`Ungültige Weltwunder-Abmessungen ${id}`);
    }

    const idle = typedSprite(row.idle, textures), death = typedSprite(row.death, textures);
    groundRows.push({id, decal, nativeSize: row.raw.size, placementProfile: 'native-unit-factory-undertexture',
      yawOffset: SOURCE_VISUAL_YAW_OFFSET,
      width: shape.width, depth: shape.depth, offsetX: shape.offsetX, offsetY: shape.offsetY,
      rural: [idle, idle, idle], urban: [idle, idle, idle],
      death: {rural: [death, death, death], urban: [death, death, death], startFadeDelay: row.deathDelay, fadeOutTime: row.deathFade},
    });
    modelRows.push({id, assetKind: 0, epoch: 0, until: 15, sourceEpoch: epoch, wonderFamily: family,
      url: model.url, sha256: model.sha256, source: {...model.source, sha256: model.sourceSha256},
      nativeScaleBinding: binding, yawOffset: SOURCE_VISUAL_YAW_OFFSET, verified: true});
    const icons = {};
    for (const profile of ['packed5', 'packed7']) {
      const icon = bodies.icons?.[profile]?.[family];
      if (!asset(icon, `icons/${profile}/${family}.png`)) throw new Error(`Fehlendes Weltwunder-Porträt ${id}`);
      icons[profile] = tagged(icon);
    }
    descriptions.set(id, {name: definition.displayName, family, icons});
  }
  return {modelRows, groundRows, descriptions};
}

export async function loadWonderPresentation({enabled, fetchFile = globalThis.fetch, base = import.meta.url} = {}) {
  const empty = {enabled: false, modelRows: [], groundRows: [], iconFor: () => null, nameFor: () => null};
  if (!enabled) return empty;
  try {
    const fetchDocument = async name => {
      const response = await fetchFile(new URL(PREFIX + name, base), {cache: 'no-store'});
      if (!response.ok) throw new Error(`Weltwunder-Katalog ${response.status}`);
      return new Uint8Array(await response.arrayBuffer());
    };
    const [bodyBytes, geometryBytes] = await Promise.all([fetchDocument('manifest.json'), fetchDocument('geometry.json')]);
    const decode = bytes => JSON.parse(new TextDecoder().decode(bytes));
    const bodies = decode(bodyBytes), geometry = decode(geometryBytes);
    if (globalThis.crypto?.subtle) {
      const bodyDigest = new Uint8Array(await crypto.subtle.digest('SHA-256', bodyBytes));
      if (Array.from(bodyDigest, byte => byte.toString(16).padStart(2, '0')).join('') !== geometry.bodyManifestSha256) {
        throw new Error('Weltwunder-Kataloge passen nicht zusammen');
      }
    }
    const {modelRows, groundRows, descriptions} = manifestRows(bodies, geometry);
    return {enabled: true, modelRows, groundRows, bodyManifestSha256: geometry.bodyManifestSha256,
      iconFor: (id, profile = 'packed7') => descriptions.get(id)?.icons[profile] ?? null,
      nameFor: id => descriptions.get(id)?.name ?? null};
  } catch (error) {
    return {...empty, error: error.message};
  }
}
