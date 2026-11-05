import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {mountSkirmishMenu} from '../docs/skirmish-menu.js';

let checks = 0;
function eq(actual, expected) {
  assert.deepEqual(actual, expected);
  checks++;
}
class Element extends EventTarget {
  constructor(id) { super(); this.id = id; }
  attributes = new Map();
  textContent = '';
  hidden = false;
  getAttribute(name) { return this.attributes.get(name); }
  setAttribute(name, value) { this.attributes.set(name, value); }
  focus() { this.focused = true; }
}
function fixture() {
  const names = ['game', 'advanced', 'map', 'environment'];
  const tabs = names.map(name => new Element(`sk-${name}-tab`));
  const panels = names.map(name => new Element(`sk-${name}`));
  const title = new Element('skirmish-title');
  const play = new Element('play');
  const playerTitle = new Element('skirmish-player-title');
  const overview = new Element('skirmish-overview');
  const epochs = new Element('sk-overview-epochs');
  const players = new Element('sk-overview-players');
  const map = new Element('sk-overview-map');
  const count = new Element('nai');
  count.value = '2';
  const selection = [['start-epoch','1 · Steinzeit'], ['end-epoch','15 · Synthetikzeitalter'],
    ['map-visibility','Unerforscht']].map(([id,textContent]) => {
    const control = new Element(id);
    control.selectedOptions = [{textContent}];
    return control;
  });
  const group = new Element('skirmish-tabs');
  group.hidden = title.hidden = true;
  group.querySelectorAll = () => tabs;
  for (const [index, tab] of tabs.entries()) tab.setAttribute('aria-controls', panels[index].id);
  const root = new Element('menu');
  const all = [group, title, play, playerTitle, overview, epochs, players, map, count, ...tabs, ...panels, ...selection];
  const classes = new Set(), styles = new Map();
  root.querySelector = selector => all.find(node => '#' + node.id === selector);
  root.classList = {add: name => classes.add(name)};
  root.style = {setProperty: (name, value) => styles.set(name, value)};
  return {root, tabs, panels, group, title, play, classes, styles, overview, epochs, players, map, count};
}

const plain = fixture();
eq(await mountSkirmishMenu(plain.root, {enabled: false, originalArt: true}), false);
eq(plain.group.hidden, true);
eq(plain.panels.every(panel => !panel.hidden), true);
const test = fixture();
const manifestUrl = new URL('../docs/local-original/skirmish-native/manifest.json', import.meta.url);
const file = await fs.readFile(manifestUrl).catch(error => {
  if (error.code !== 'ENOENT') throw error;
  return null;
});
const loaded = await mountSkirmishMenu(test.root, {enabled: true, originalArt: !!file, fetchFile: async url => {
  eq(url.href, manifestUrl.href);
  return {ok: true, json: async () => JSON.parse(file)};
}});
eq(loaded, !!file);
eq([...test.classes], file ? ['skirmish-layout', 'skirmish-original-art'] : ['skirmish-layout']);
eq(test.group.hidden, false);
eq(test.title.hidden, false);
eq(test.overview.hidden, false);
eq([test.epochs.textContent,test.players.textContent,test.map.textContent],
  ['1 · Steinzeit bis 15 · Synthetikzeitalter', '3', 'Unerforscht']);
test.count.value = '3';
test.root.dispatchEvent(new Event('change'));
eq(test.players.textContent, '4');
eq(test.panels.map(panel => panel.hidden), [false, true, true, true]);
eq(test.panels.map(panel => panel.getAttribute('role')), Array(4).fill('tabpanel'));
test.tabs[2].dispatchEvent(new Event('click'));
eq(test.tabs.map(tab => tab.tabIndex), [-1, -1, 0, -1]);
eq(test.panels.map(panel => panel.hidden), [true, true, false, true]);
function key(tab, name) {
  const event = new Event('keydown', {cancelable: true});
  Object.defineProperty(event, 'key', {value: name});
  tab.dispatchEvent(event);
  eq(event.defaultPrevented, true);
}
key(test.tabs[2], 'End');
eq(test.tabs.map(tab => tab.getAttribute('aria-selected')), ['false', 'false', 'false', 'true']);
key(test.tabs[3], 'ArrowRight');
eq(test.tabs[0].focused, true);
eq(test.panels.map(panel => panel.hidden), [false, true, true, true]);
if (file) {
  eq(test.tabs.map(tab => tab.textContent), ['Spiel', 'Erweitert', 'Karte', 'Umgebung']);
  eq(test.play.textContent, 'Starten');
  eq(test.title.textContent, 'Einzelsp.-Scharmützel');
  eq(test.styles.size, 23);
  for (const style of test.styles.values()) {
    const url = new URL(style.slice(5, -2));
    eq((await fs.readFile(url)).subarray(1, 4).toString('ascii'), 'PNG');
  }
}
const missing = fixture();
eq(await mountSkirmishMenu(missing.root, {enabled: true, originalArt: true, fetchFile: async () => ({ok: false})}), false);
eq([...missing.classes], ['skirmish-layout']);
console.log(JSON.stringify({status: 'passed', checks, sourceArtChecked: !!file,
  scope: 'Real skirmish controller and source-art binding with substituted DOM.'}));
