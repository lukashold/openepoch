import {calculateHudLayout, nativeMetaEpoch} from './original-hud-layout.js';
import {sourceButtonSprite} from './original-diplomacy-layout.js';

const PANE = 'fullmap_warplanmanager';
const CONTROLS = Object.freeze({create: 'fullmap_warplan_create', select: 'fullmap_warplan_select',
  path: 'fullmap_warplan_addpath', area: 'fullmap_warplan_addarea', target: 'fullmap_warplan_addpoint',
  label: 'fullmap_warplan_addlabel', delete: 'fullmap_warplan_delete', copy: 'fullmap_warplan_copy',
  send: 'fullmap_warplan_send', sendAllies: 'fullmap_warplan_send_allies', accept: 'fullmap_warplan_accept', reject: 'fullmap_warplan_reject'});

// A source widget subset with no dynamic recipient/palette instantiation.
export function originalWarplanLayout(art, width, height) {
  const {source} = art, widgets = {};
  function add(name) {
    if (!name || widgets[name]) return;
    const row = source.widgets[name];
    if (!row) throw new Error('Missing original War Plans widget ' + name);
    widgets[name] = row;
    for (const dependency of [row.PARENT, row.ANCHORNAME1, row.ANCHORNAME2]) add(dependency);
  }
  for (const name of [PANE, 'fullmap_warplan_map_label', 'fullmap_warplan_buttons_divider', ...Object.values(CONTROLS)]) add(name);
  return calculateHudLayout({widgets, frameStyles: source.frameStyles}, width, height);
}

export function originalWarplanControls(art, epoch) {
  const {source} = art, meta = nativeMetaEpoch(epoch);
  return Object.fromEntries(Object.entries(CONTROLS).map(([action, name]) => {
    const row = source.widgets[name];
    const sprites = Object.fromEntries(['normal','hover','pressed','pressedHover'].map(state =>
      [state, art.sprite(sourceButtonSprite(source, row.SPRITE.toLowerCase(), meta, state))]));
    return [action, {name, label: source.labels[row.LABEL]?.text ?? source.labels[row.TOOLTIP]?.text ?? '',
      command: row.CMDSTR.replace(/^"|"$/g, ''), sprites}];
  }));
}
