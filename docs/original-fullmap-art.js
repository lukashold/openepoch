import {calculateHudLayout, nativeMetaEpoch} from './original-hud-layout.js';

const FORMAT = 'ee2-native-fullmap-visuals-v1';
const MANIFEST = './local-original/fullmap-native/manifest.json';
const PACKS = ['packed7', 'packed5'];
const SHELL = new Set(['fullmap_root', 'fullmap_map', 'fullmap_rightbar', 'fullmap_rightbar_bkg1', 'fullmap_rightbar_bkg2',
  'fullmap_rightbar_usable', 'fullmap_close', 'fullmap_tabs_container', 'fullmap_pane_container', 'fullmap_PIP']);

// Resolve the exact registry rows for the caller's selected asset profile.
export async function loadOriginalFullmapArt(enabled, {pack, fetchFile = globalThis.fetch,
  url = new URL(MANIFEST, import.meta.url)} = {}) {
  if (!enabled) return null;
  if (pack !== undefined && !PACKS.includes(pack)) throw new RangeError('Unknown original full-map art profile');
  const response = await fetchFile(url);
  if (!response.ok) return null;
  const catalog = await response.json();
  if (catalog.format !== FORMAT || catalog.preferred !== PACKS[0] || catalog.fallback !== PACKS[1]) return null;
  const profile = pack ?? catalog.preferred;
  const source = catalog.packs?.[profile];
  if (!source?.art || !source.sprites || !source.widgets || !source.tabs) return null;
  const base = new URL('../../', url);
  const sprite = name => {
    const row = source.sprites[name?.toLowerCase()];
    if (!row) return null;
    const asset = new URL(row.url, base);
    asset.searchParams.set('v', row.sha256.slice(0, 16));
    return {...row, url: asset.href};
  };
  const variant = (family, epoch, state) => {
    if (!Number.isInteger(epoch) || epoch < 0 || epoch > 14) throw new RangeError('Invalid full-map art epoch');
    const entry = family?.[nativeMetaEpoch(epoch) - 1];
    return sprite(typeof entry === 'string' ? entry : entry?.[state]);
  };
  return Object.freeze({
    profile, source, sprite,
    role: (role, epoch, state = 'normal') => variant(source.art[role], epoch, state),
    tabs: (mode, epoch, state = 'normal') => (source.tabs[mode] || []).map(tab => ({
      widget: tab.widget, name: tab.name, pane: tab.pane, label: source.labels[tab.tooltip]?.text ?? '',
      sprite: variant(tab.sprites, epoch, state),
    })),
    frame: (style, epoch) => {
      const meta = nativeMetaEpoch(epoch);
      const entry = source.frameSprites[meta === 1 ? style : `${style}_e${meta}`] ?? source.frameSprites[style];
      return entry ? Object.fromEntries(Object.entries(entry).map(([part, key]) => [part, sprite(key)])) : null;
    },
    referenceLayout: ({width, height, tabs = 'wide'} = {}) => {
      if (!Object.hasOwn(source.tabs, tabs)) throw new RangeError('Invalid native full-map tab family');
      const widgets = Object.fromEntries(Object.entries(source.widgets).filter(([name]) =>
        SHELL.has(name) || name === 'fullmap_options' || name.startsWith('fullmap_options_') || name.startsWith('fullmap_pip_')));
      // The widget table declares these templates are renamed when created.
      for (const {widget, name} of source.tabs[tabs]) widgets[name] = {...source.widgets[widget], NAME: name};
      const [referenceWidth, referenceHeight] = catalog.coordinateReference;
      return calculateHudLayout({widgets, frameStyles: source.frameStyles, resolutionWidths: catalog.resolutionWidths},
        width ?? referenceWidth, height ?? referenceHeight);
    },
  });
}
