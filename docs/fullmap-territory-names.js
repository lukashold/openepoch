import {bitmapTextLayout} from './original-fonts.js';

const TERRITORY_WORDS = 6;
const CITY_BYTES = 16;
const TERRITORY = {font:'fnt_pal_large_outline', color:[210,210,210], bold:false,
  fallback:['Nicht beanspruchtes Territorium:', 'Unbenanntes Territorium']};
const CITY = {font:'fnt_pal_large_bold_outline', color:[255,255,255], bold:true, fallback:['Stadtzentrum']};

async function loadNameCatalog(kind, version, fetchFile) {
  if (!version) return null;
  try {
    const url = new URL(`./local-original/fullmap-native/${kind}-names.json`, import.meta.url);
    const response = await fetchFile(url);
    if (!response.ok) return null;
    const catalog = await response.json();
    if (catalog.format !== `ee2-fullmap-${kind}-names-v1` || catalog.catalog !== version
        || !Array.isArray(catalog.names) || catalog.names.length <= 2
        || catalog.names.some(name => typeof name !== 'string' || !name)) return null;
    return catalog;
  } catch {
    return null;
  }
}

export function loadFullmapTerritoryNameCatalog(wasm, fetchFile = globalThis.fetch) {
  return loadNameCatalog('territory', wasm.territory_names_catalog_version?.() >>> 0, fetchFile);
}

export function loadFullmapCityNameCatalog(wasm, fetchFile = globalThis.fetch) {
  return loadNameCatalog('city', wasm.city_names_catalog_version?.() >>> 0, fetchFile);
}

export function readFullmapTerritoryLabels(wasm) {
  const count = wasm.fullmap_territory_labels_build();
  const pointer = wasm.fullmap_territory_labels_ptr();
  const records = new Uint32Array(wasm.memory.buffer, pointer, count * TERRITORY_WORDS);
  const labels = [];
  for (let i = 0; i < records.length; i += TERRITORY_WORDS) {
    const [id, minX, minY, maxX, maxY, name] = records.subarray(i, i + TERRITORY_WORDS);
    labels.push({id, x:(minX + maxX) / 2, y:(minY + maxY) / 2, name});
  }
  return labels;
}

export function readFullmapCityLabels(wasm) {
  const count = wasm.fullmap_city_labels_build();
  const pointer = wasm.fullmap_city_labels_ptr();
  const records = new DataView(wasm.memory.buffer, pointer, count * CITY_BYTES);
  const labels = [];
  for (let i = 0; i < records.byteLength; i += CITY_BYTES) {
    labels.push({handle:records.getUint32(i,true), x:records.getFloat32(i+4,true),
      y:records.getFloat32(i+8,true), name:records.getUint32(i+12,true)});
  }
  return labels;
}

const clampLabel = (point, extent, half) => half * 2 >= extent ? extent / 2
  : Math.max(half, Math.min(extent - half, point));

function paintLabels(draw, labels, project, layout, style, {names = style.fallback, bitmapFonts} = {}) {
  const {width, height, viewportWidth, viewportHeight} = layout;
  const textHeight = Math.max(1, Math.round(viewportHeight * 32 / 1200));
  const halfHeight = Math.floor(textHeight / 2);
  const padding = Math.round(viewportWidth * 20 / 1600);
  const tiers = bitmapFonts?.manifest.resolutionWidths;
  const tier = tiers?.reduce((best, value, i) =>
    Math.abs(value - viewportWidth) < Math.abs(tiers[best] - viewportWidth) ? i : best, 0);
  const font = bitmapFonts?.manifest.bindings[style.font]?.[tier] ? bitmapFonts.font(style.font, tier) : null;
  draw.save();
  draw.font = `${style.bold ? 'bold ' : ''}${Math.max(1, Math.round(textHeight * 0.55))}px Arial, sans-serif`;
  draw.textAlign = 'center';
  draw.textBaseline = 'middle';
  draw.lineWidth = 2;
  draw.strokeStyle = '#000';
  draw.fillStyle = `rgb(${style.color.join(' ')})`;
  for (const label of labels) {
    const name = names[label.name] ?? style.fallback.at(-1);
    const [x, y] = project(label.x, label.y);
    const bitmap = font ? bitmapTextLayout(font, name) : null;
    const useBitmap = bitmap && !bitmap.unsupported.length;
    const textWidth = Math.min(width, (useBitmap ? bitmap.width : draw.measureText(name).width) + padding);
    const halfWidth = Math.floor(textWidth / 2);
    const left = Math.trunc(clampLabel(x, width, halfWidth) - halfWidth);
    const top = Math.trunc(clampLabel(y, height, halfHeight) - halfHeight);
    if (useBitmap) {
      bitmapFonts.draw(draw, font, name, {x:left, y:top, width:textWidth, height:textHeight,
        color:style.color, align:'center', vertical:'middle'});
    } else {
      draw.strokeText(name, left + halfWidth, top + halfHeight, width);
      draw.fillText(name, left + halfWidth, top + halfHeight, width);
    }
  }
  draw.restore();
}

export function paintFullmapTerritoryLabels(draw, labels, project, layout, options) {
  paintLabels(draw, labels, project, layout, TERRITORY, options);
}

export function paintFullmapCityLabels(draw, labels, project, layout, options) {
  paintLabels(draw, labels, project, layout, CITY, options);
}
