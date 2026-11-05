const MANIFEST = new URL('./local-original/skirmish-native/manifest.json', import.meta.url);
const TAB_WIDGETS = ['sk_gameinfo_gameset_tab', 'sk_gameinfo_advset_tab', 'sk_gameinfo_mapset_tab', 'sk_gameinfo_envset_tab'];
const PANEL_SPRITES = {
  'player-left': 'sk_gameinfo_plyrsPan_bkg1',
  'player-middle': 'sk_gameinfo_plyrsPan_bkg2',
  'player-right': 'sk_gameinfo_plyrsPan_bkg3',
  'settings-left': 'sk_gameinfo_settingsPan_bkg1',
  'settings-middle': 'sk_gameinfo_settingsPan_bkg2',
  divider: 'sk_gameinfo_separator',
  overview: 'sk_output',
};

function applyArt(root, manifest, tabs) {
  if (manifest.format !== 'ee2-native-skirmish-frontend-v1') throw Error('Unsupported skirmish artwork');
  const image = sprite => {
    const url = manifest.sprites[sprite]?.url;
    if (!/^local-original\/skirmish-native\/sprites\/[A-Za-z0-9_]+\.png$/.test(url)) throw Error('Unbound skirmish sprite');
    return `url("${new URL('./' + url, import.meta.url).href}")`;
  };
  const assets = Object.fromEntries(Object.entries(PANEL_SPRITES)
    .map(([part, widget]) => [part, image(manifest.widgets[widget].SPRITE)]));
  for (const [part, widget] of [['tab', TAB_WIDGETS[0]], ['action', 'sk_launch_button']]) {
    const names = manifest.spriteStates[manifest.widgets[widget].SPRITE];
    if (names?.length !== 4) throw Error('Incomplete skirmish button states');
    for (const [index, state] of ['normal', 'hover', 'active', 'active-hover'].entries()) {
      assets[part + '-' + state] = image(names[index]);
    }
  }
  const frame = manifest.frameSprites.wfs_fe_panel;
  for (const [index, part] of ['ul', 'll', 'lr', 'ur', 'top', 'left', 'bottom', 'right'].entries()) {
    assets['frame-' + part] = image(frame[index]);
  }
  const label = widget => manifest.localizedText[manifest.widgets[widget].LABEL]?.text;
  for (const [index, widget] of TAB_WIDGETS.entries()) tabs[index].textContent = label(widget) || tabs[index].textContent;
  const heading = root.querySelector('#skirmish-title');
  const start = root.querySelector('#play');
  heading.textContent = label('sk_title') || heading.textContent;
  if (!start.disabled) start.textContent = label('sk_launch_button') || start.textContent;
  for (const [part, url] of Object.entries(assets)) root.style.setProperty('--sk-' + part, url);
  root.classList.add('skirmish-original-art');
}

export function mountSkirmishMenu(root, {enabled, originalArt, fetchFile = globalThis.fetch} = {}) {
  if (!enabled) return Promise.resolve(false);
  const group = root.querySelector('#skirmish-tabs');
  const tabs = [...group.querySelectorAll('[role="tab"]')];
  const panels = tabs.map(tab => root.querySelector('#' + tab.getAttribute('aria-controls')));
  function select(active, focus = false) {
    for (const [index, tab] of tabs.entries()) {
      const selected = index === active;
      tab.setAttribute('aria-selected', String(selected));
      tab.tabIndex = selected ? 0 : -1;
      panels[index].hidden = !selected;
    }
    if (focus) tabs[active].focus();
  }
  for (const [index, tab] of tabs.entries()) {
    const panel = panels[index];
    panel.setAttribute('role', 'tabpanel');
    panel.setAttribute('aria-labelledby', tab.id);
    tab.addEventListener('click', () => select(index));
    tab.addEventListener('keydown', event => {
      const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0;
      let target;
      if (step) target = (index + step + tabs.length) % tabs.length;
      else if (event.key === 'Home') target = 0;
      else if (event.key === 'End') target = tabs.length - 1;
      else return;
      event.preventDefault();
      select(target, true);
    });
  }
  select(0);
  group.hidden = false;
  root.querySelector('#skirmish-title').hidden = false;
  root.querySelector('#skirmish-player-title').hidden = false;
  const overview = root.querySelector('#skirmish-overview');
  const selected = id => root.querySelector('#' + id)?.selectedOptions?.[0]?.textContent ?? '';
  function updateOverview() {
    root.querySelector('#sk-overview-epochs').textContent = [selected('start-epoch'), selected('end-epoch')].join(' bis ');
    root.querySelector('#sk-overview-players').textContent = String(1 + Number(root.querySelector('#nai').value));
    root.querySelector('#sk-overview-map').textContent = selected('map-visibility');
  }
  root.addEventListener('change', updateOverview);
  updateOverview();
  overview.hidden = false;
  root.classList.add('skirmish-layout');
  if (!originalArt) return Promise.resolve(false);
  return fetchFile(MANIFEST, {cache: 'no-cache'}).then(response => {
    if (!response.ok) return false;
    return response.json().then(manifest => {
      applyArt(root, manifest, tabs);
      return true;
    });
  }).catch(() => false);
}
