const profiles = {packed5: [77, 60], packed7: [308, 240]};
const starts = [0, 3, 6, 9, 12, 14, 15];
const regions = [0, 0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3];

export function cityCenterIconCatalog(manifest, {profile, neutralFallback = null} = {}) {
  if (typeof profile !== 'string' || !Object.hasOwn(profiles, profile) || ![null, 'west'].includes(neutralFallback)) {
    throw Error('CityCenter icon profile must be explicit');
  }
  const [width, height] = profiles[profile];
  const binding = manifest?.profiles?.[profile];
  if (manifest?.format !== 'native-citycenter-icons-v1' || binding?.rows?.length !== 24
      || !Array.isArray(binding.rows) || !binding.assets || binding.scale !== width / 77) {
    throw Error('Invalid CityCenter icon manifest');
  }
  const rows = new Map();
  const assets = new Map();
  const directory = `local-original/citycenter-icons/${profile}/`;
  for (const row of binding.rows) {
    const {epoch, until, region, asset, civilizations} = row;
    const end = starts[starts.indexOf(epoch) + 1];
    const expected = regions.flatMap((value, index) => value === region ? [index] : []);
    const key = `${region}:${epoch}`;
    const image = Object.hasOwn(binding.assets, asset) ? binding.assets[asset] : null;
    if (!starts.slice(0, -1).includes(epoch) || until !== end || !expected.length
        || !Array.isArray(civilizations) || civilizations.length !== expected.length
        || expected.some((value, index) => civilizations[index] !== value) || rows.has(key)
        || typeof asset !== 'string' || !/^[a-z0-9_]+$/.test(asset) || image?.url !== `${directory}${asset}.png`
        || image.width !== width || image.height !== height || !/^[a-f0-9]{64}$/.test(image.sha256)) {
      throw Error('Invalid CityCenter icon row');
    }
    const info = Object.freeze({url: image.url, width, height, sha256: image.sha256});
    rows.set(key, Object.freeze({epoch, until, region, asset, selectedName: row.selectedName, image: info}));
    assets.set(info.url, info);
  }
  return Object.freeze({
    files: Object.freeze([...assets.values()]),
    lookup(epoch, civilization) {
      if (!Number.isInteger(epoch) || epoch < 0 || epoch >= 15 || !Number.isInteger(civilization)) return null;
      const westernFallback = civilization === 14 && neutralFallback === 'west';
      const region = westernFallback ? 0 : regions[civilization === -1 ? 0 : civilization];
      if (region === undefined) return null;
      const start = starts[starts.findIndex(value => epoch < value) - 1];
      const row = rows.get(`${region}:${start}`);
      return westernFallback ? Object.freeze({...row, ownNeutralFallback: true}) : row;
    },
  });
}

export async function loadCityCenterIcons({
  enabled = false, profile, neutralFallback = null,
  fetchFile = globalThis.fetch, bitmapFile = globalThis.createImageBitmap,
  manifestURL = new URL('./local-original/citycenter-icons/manifest.json', import.meta.url),
  base = new URL('./', import.meta.url),
} = {}) {
  const images = new Map();
  const errors = [];
  let catalog = null;
  const result = Object.freeze({
    icon(epoch, civilization) {
      const row = catalog?.lookup(epoch, civilization);
      return images.get(row?.image.url);
    },
    status: () => ({enabled: catalog !== null, profile, images: images.size, errors: [...errors]}),
    close() {
      for (const image of images.values()) image.close?.();
      images.clear();
      catalog = null;
    },
  });
  if (!enabled) return result;
  try {
    const response = await fetchFile(manifestURL, {cache: 'no-cache'});
    if (!response.ok) throw Error('CityCenter icon manifest unavailable');
    catalog = cityCenterIconCatalog(await response.json(), {profile, neutralFallback});
    await Promise.all(catalog.files.map(async image => {
      try {
        const url = new URL(image.url, base);
        url.searchParams.set('v', image.sha256.slice(0, 16));
        const response = await fetchFile(url, {cache: 'force-cache'});
        if (!response.ok) throw Error('CityCenter icon unavailable: ' + image.url);
        const bitmap = await bitmapFile(await response.blob(), {premultiplyAlpha: 'none', colorSpaceConversion: 'none'});
        if (bitmap.width !== image.width || bitmap.height !== image.height) {
          bitmap.close?.();
          throw Error('CityCenter icon dimensions differ: ' + image.url);
        }
        images.set(image.url, bitmap);
      } catch (error) {
        errors.push(error.message);
      }
    }));
  } catch (error) {
    errors.push(error.message);
  }
  return result;
}
