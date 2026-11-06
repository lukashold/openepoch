import {bitmapTextLayout} from './original-fonts.js';

const RECORD_WORDS = 6;
const FALLBACK_NAMES = ['Nicht beanspruchtes Territorium:', 'Unbenanntes Territorium'];
const TERRITORY_COLOR = [210,210,210];

export async function loadFullmapTerritoryNameCatalog(wasm, fetchFile = globalThis.fetch) {
  const version = wasm.territory_names_catalog_version?.() >>> 0;
  if (!version) return null;
  try {
    const url = new URL('./local-original/fullmap-native/territory-names.json', import.meta.url);
    const response = await fetchFile(url);
    if (!response.ok) return null;
    const catalog = await response.json();
    if (catalog.format !== 'ee2-fullmap-territory-names-v1' || catalog.catalog !== version
        || !Array.isArray(catalog.names) || catalog.names.length <= 2
        || catalog.names.some(name => typeof name !== 'string' || !name)) return null;
    return catalog;
  } catch {
    return null;
  }
}

export function readFullmapTerritoryLabels(wasm) {
  const count = wasm.fullmap_territory_labels_build();
  const pointer = wasm.fullmap_territory_labels_ptr();
  const records = new Uint32Array(wasm.memory.buffer, pointer, count * RECORD_WORDS);
  const labels = [];
  for (let i = 0; i < records.length; i += RECORD_WORDS) {
    const [id, minX, minY, maxX, maxY, name] = records.subarray(i, i + RECORD_WORDS);
    labels.push({id, x:(minX + maxX) / 2, y:(minY + maxY) / 2, name});
  }
  return labels;
}

const clampLabel = (point, extent, half) => half * 2 >= extent ? extent / 2
  : Math.max(half, Math.min(extent - half, point));

export function paintFullmapTerritoryLabels(draw, labels, project, layout, {names = FALLBACK_NAMES, bitmapFonts} = {}) {
  const {width, height, viewportWidth, viewportHeight} = layout;
  const textHeight = Math.max(1, Math.round(viewportHeight * 32 / 1200));
  const halfHeight = Math.floor(textHeight / 2);
  const padding = Math.round(viewportWidth * 20 / 1600);
  const tiers = bitmapFonts?.manifest.resolutionWidths;
  const tier = tiers?.reduce((best, value, i) =>
    Math.abs(value - viewportWidth) < Math.abs(tiers[best] - viewportWidth) ? i : best, 0);
  const font = bitmapFonts?.manifest.bindings.fnt_pal_large_outline?.[tier]
    ? bitmapFonts.font('fnt_pal_large_outline', tier) : null;
  draw.save();
  draw.font = `${Math.max(1, Math.round(textHeight * 0.55))}px Arial, sans-serif`;
  draw.textAlign = 'center';
  draw.textBaseline = 'middle';
  draw.lineWidth = 2;
  draw.strokeStyle = '#000';
  draw.fillStyle = 'rgb(210 210 210)';
  for (const label of labels) {
    const name = names[label.name] ?? FALLBACK_NAMES[1];
    const [x, y] = project(label.x, label.y);
    const bitmap = font ? bitmapTextLayout(font, name) : null;
    const useBitmap = bitmap && !bitmap.unsupported.length;
    const textWidth = Math.min(width, (useBitmap ? bitmap.width : draw.measureText(name).width) + padding);
    const halfWidth = Math.floor(textWidth / 2);
    const left = Math.trunc(clampLabel(x, width, halfWidth) - halfWidth);
    const top = Math.trunc(clampLabel(y, height, halfHeight) - halfHeight);
    if (useBitmap) {
      bitmapFonts.draw(draw, font, name, {x:left, y:top, width:textWidth, height:textHeight,
        color:TERRITORY_COLOR, align:'center', vertical:'middle'});
    } else {
      draw.strokeText(name, left + halfWidth, top + halfHeight, width);
      draw.fillText(name, left + halfWidth, top + halfHeight, width);
    }
  }
  draw.restore();
}
