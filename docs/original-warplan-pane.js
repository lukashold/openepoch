import {originalWarplanControls,originalWarplanLayout} from './original-warplan-layout.js';
import {createOriginalWarplanPath,originalWarplanPathWidths,visitOriginalWarplanPaths} from './original-warplan-path.js';
import {createWarplanPeerControls} from './original-warplan-peer-controls.js';
import {visitOriginalWarplanGeometry} from './original-warplan-geometry.js?v=native24';

export function createOriginalWarplanPane({host,options,refresh,playCue,peer,recipient,clock,teams}) {
  const editor = createOriginalWarplanPath({playCue});
  const element = (tag, name) => {
    const node = document.createElement(tag);
    node.className = name;
    if (tag === 'button') node.type = 'button';
    node.hidden = true;
    host.append(node);
    return node;
  };
  const tools = element('div','fullmap-warplan-tools');
  const optionsTab = element('button','fullmap-warplan-button');
  const warplansTab = element('button','fullmap-warplan-button');
  const create = element('button','fullmap-warplan-button');
  const path = element('button','fullmap-warplan-button');
  let open = false;
  let enabled = false;
  let layoutKey = '';

  const rect = (node, {x,y,width,height}) => Object.assign(node.style,{left:`${x}px`,top:`${y}px`,width:`${width}px`,height:`${height}px`});
  const background = sprite => sprite ? `url("${sprite.url}")` : 'none';

  function button(node, geometry, label, sprites, pressed, text = '') {
    rect(node,geometry);
    node.textContent = text;
    node.title = label;
    node.setAttribute('aria-label',label);
    node.setAttribute('aria-pressed',String(pressed));
    node.style.backgroundImage = background(sprites[pressed?'pressed':'normal']);
    node.style.setProperty('--rollover',background(sprites[pressed?'pressedHover':'hover']));
  }
  const exchange=createWarplanPeerControls({element,button,rect,editor,peer,recipient,clock,refresh});

  function select(openPane) {
    if (!enabled || open === openPane) return false;
    open = openPane;
    if(!open)exchange.close();
    refresh();
    return true;
  }

  function start() {
    if (!open || !exchange.editing() || !editor.create()) return false;
    refresh();
    return true;
  }

  warplansTab.onclick = () => select(true);
  optionsTab.onclick = () => select(false);
  create.onclick = start;

  function layout(art, width, height, epoch) {
    enabled = !!art && width >= 800 && height >= 600;
    const shown = enabled && open;
    exchange.layout(art,width,height,epoch,shown);
    const key = `${art?.profile}:${width}:${height}:${epoch}:${shown}:${editor.hasDraft()}:${exchange.editing()}`;
    if (key === layoutKey) return;
    layoutKey = key;
    optionsTab.hidden = warplansTab.hidden = !enabled;
    options.hidden = shown;
    tools.hidden = create.hidden = path.hidden = !shown;
    if (!enabled) return;

    const tabs = art.referenceLayout({width,height});
    const tabStates = Object.fromEntries(['normal','hover','pressed','pressedHover'].map(state =>
      [state,art.tabs('wide',epoch,state)]));
    for (const [node,pane,pressed] of [[optionsTab,'fullmap_options',!shown],[warplansTab,'fullmap_warplanmanager',shown]]) {
      const sprites = Object.fromEntries(Object.entries(tabStates).map(([state,rows]) => [state,rows.find(row => row.pane===pane)]));
      const tab = sprites.normal;
      button(node,tabs.get(tab.name),tab.label,Object.fromEntries(Object.entries(sprites).map(([key,row])=>[key,row.sprite])),pressed);
    }
    if (!shown) return;

    const controls = originalWarplanControls(art,epoch);
    const positions = originalWarplanLayout(art,width,height);
    rect(tools,positions.get('fullmap_warplan_tools_cont'));
    tools.style.backgroundImage = background(art.role('panelInset',epoch));
    button(create,positions.get(controls.create.name),controls.create.label,controls.create.sprites,false,controls.create.label);
    button(path,positions.get(controls.path.name),controls.path.label,controls.path.sprites,editor.hasDraft());
    create.disabled = editor.hasDraft() || !exchange.editing();
    path.disabled = !editor.hasDraft() || !exchange.editing();
  }

  function pointer(kind, event, point) {
    if (!enabled || !open || !editor.hasDraft() || !exchange.editing() || event.isPrimary === false) return false;
    const changed = kind === 'move' ? editor.move(point) :
      event.button === 2 ? editor.finish() : event.button === 0 && editor.append(point);
    if (changed) refresh();
    return kind === 'move' || event.button === 0 || event.button === 2;
  }

  function key(event) {
    if (!enabled || !open) return false;
    if(exchange.key(event))return true;
    if (!event.ctrlKey || event.altKey || event.metaKey || event.shiftKey || event.key.toLowerCase() !== 'n') return false;
    event.preventDefault();
    if (!event.repeat) start();
    return true;
  }

  function paint(draw, project, screenWidth, screenHeight) {
    if (!enabled || !open) return;
    const {line,stroke} = originalWarplanPathWidths(screenWidth);
    draw.save();
    visitOriginalWarplanPaths(exchange.visiblePaths(),project,(points,colorPlayer) => {
      const rgb=teams?.[colorPlayer-1];
      const color=rgb ? `rgb(${rgb.map(value=>Math.round(value*255)).join(',')})` : '#fff';
      visitOriginalWarplanGeometry(points,line,screenHeight,stroke,(polygon,edge) => {
        draw.beginPath();
        draw.moveTo(...polygon[0]);
        for (let i=1; i<polygon.length; i++) draw.lineTo(...polygon[i]);
        draw.closePath();
        // The original edge uses black RGB with both DX8 alpha operations disabled.
        draw.fillStyle = edge ? '#000' : color;
        draw.fill();
      });
    });
    draw.restore();
  }

  return Object.freeze({layout, pointer, paint, key, start, select,
    capturing: () => enabled && open && editor.hasDraft() && exchange.editing(), snapshot: editor.snapshot,
    modal:exchange.modal,close:exchange.close,commit:()=>{editor.reset();refresh();},refresh,
    reset: () => { editor.reset(); exchange.reset(); open = false; refresh(); }});
}
