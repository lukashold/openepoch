// EE2-profile economy layout. Slots follow the simulation API, independently of
// the original display order. Moving existing controls keeps their listeners.
export const RESOURCE_KEYS=Object.freeze(['food','wood','iron','stone','gold','tin','saltpeter','oil','uranium']);
export const RESOURCE_NAMES=Object.freeze(['Nahrung','Holz','Eisen','Stein','Gold','Zinn','Salpeter','Öl','Uran']);
const DISPLAY_ORDER=[0,1,3,4,5,2,6,7,8];
const bars=new WeakMap();
const RESOURCE_HELP=[
 'Nahrung für Bürger und Einheiten. Bürger sammeln sie an Nahrungsvorkommen und auf Feldern.',
 'Holz für Gebäude, Schiffe und Ausrüstung. Bürger fällen Bäume.',
 'Eisen: Sonderrohstoff der Epochen 4–9. Bürger bauen es am Eisenvorkommen ab.',
 'Stein für Gebäude und Befestigungen. Bürger bauen ihn am Steinvorkommen ab.',
 'Gold für Einheiten, Forschung und Ausbau. Bürger bauen es am Goldvorkommen ab.',
 'Zinn: Sonderrohstoff der Epochen 1–6. Bürger bauen es am Zinnvorkommen ab.',
 'Salpeter: Sonderrohstoff der Epochen 7–12. Bürger bauen ihn am Salpetervorkommen ab.',
 'Öl: Sonderrohstoff ab Epoche 10. Bürger fördern es in einem Bohrturm auf einem Ölvorkommen.',
 'Uran: Sonderrohstoff ab Epoche 13. Bürger fördern es in einer Uranmine auf einem Uranvorkommen.'
];
const NOOP=Object.freeze({update(){}});

function epochResources(epoch=0){
 const active=Array(9).fill(false);
 for(const slot of [0,1,3,4])active[slot]=true;
 const originalEpoch=Number.isFinite(+epoch)?+epoch+1:1;
 const special=originalEpoch<4?[5]:originalEpoch<7?[5,2]:originalEpoch<10?[2,6]:originalEpoch<13?[6,7]:[7,8];
 for(const slot of special)active[slot]=true;
 return active;
}

function installStyle(){
 if(document.getElementById('ee2-economy-style'))return;
 const style=document.createElement('style');style.id='ee2-economy-style';
 style.textContent=`
 #top.ee2-economy-top{height:66px;display:flex;flex-direction:column;align-items:stretch;justify-content:center;gap:2px;padding:3px 10px 4px;overflow:hidden;background-color:#abb09c;color:#172114;text-shadow:0 1px #e8ecda88;border-bottom:1px solid #48523e;box-shadow:0 3px 12px #0008;font-variant-numeric:tabular-nums}
 #top.ee2-economy-top .ee2-economy-first{display:flex;align-items:center;gap:10px;min-height:27px;flex:1;min-width:0}
 #top.ee2-economy-top .ee2-economy-resources{display:flex;align-items:center;gap:0;flex:0 1 auto;min-width:0}
 #top.ee2-economy-top .ee2-economy-resource{display:inline-flex;align-items:center;gap:5px;min-width:0;padding:1px 9px 1px 2px;border-right:1px solid #59664c55;white-space:nowrap}
 #top.ee2-economy-top .ee2-economy-resource[hidden]{display:none}
 #top.ee2-economy-top .ee2-economy-resource .resource-icon{display:block;flex:none;width:22px;height:22px;object-fit:contain;filter:drop-shadow(0 1px 1px #0005)}
 #top.ee2-economy-top .ee2-economy-name{font-size:13px;font-weight:500;color:#263021}
 #top.ee2-economy-top .ee2-economy-resource b{font-size:13px;line-height:22px;font-weight:700;min-width:3ch;text-align:right;color:#11190d}
 #top.ee2-economy-top .ee2-economy-summary{display:flex;align-items:center;justify-content:flex-end;gap:12px;flex:1;min-width:0;font-size:13px}
 #top.ee2-economy-top .ee2-economy-summary>.r{flex:none;gap:5px}
 #top.ee2-economy-top .ee2-economy-summary .ee2-economy-epoch{flex:0 1 auto;min-width:0;overflow:hidden;text-overflow:ellipsis}
 #top.ee2-economy-top #epoch{display:block;overflow:hidden;text-overflow:ellipsis;font-size:12px;letter-spacing:0;color:#172114}
 #top.ee2-economy-top #science{min-width:4.5em;color:#172114;white-space:nowrap}
 #top.ee2-economy-top .ee2-economy-controls{display:flex;align-items:center;gap:5px;min-width:0;min-height:26px;font-size:12px;border-top:1px solid #6e795a66;padding-top:2px}
 #top.ee2-economy-top .ee2-economy-controls .r{gap:4px;font-size:11px}
 #top.ee2-economy-top .ee2-economy-controls .sp{flex:1;min-width:0}
 #top.ee2-economy-top .ee2-economy-controls button{flex:none;height:23px;line-height:19px;padding:1px 8px;border-radius:2px;border-color:#515b46;background-color:#a0a991;color:#15200f;font:inherit;box-shadow:inset 0 1px #e0e7d17a}
 #top.ee2-economy-top .ee2-economy-controls button:hover{border-color:#edf0cf;filter:brightness(1.09)}
 #top.ee2-economy-top #perf{flex:1 1 100px;min-width:65px;max-width:380px;color:#35412d;font-size:10px;text-align:right;text-shadow:none}
 .ee2-economy #toast{top:75px}
 @media(max-width:1100px){#top.ee2-economy-top{padding-left:6px;padding-right:6px}#top.ee2-economy-top .ee2-economy-resource{padding-right:6px;gap:4px}#top.ee2-economy-top .ee2-economy-summary{gap:7px}#top.ee2-economy-top .ee2-economy-controls{gap:4px}#top.ee2-economy-top .ee2-economy-controls button{padding-left:6px;padding-right:6px}}
 @media(max-width:900px){#top.ee2-economy-top .ee2-economy-name{display:none}#top.ee2-economy-top .ee2-economy-first{gap:6px}#top.ee2-economy-top .ee2-economy-controls{overflow-x:auto;scrollbar-width:none}#top.ee2-economy-top #perf{display:none}}
 `;
 document.head.append(style);
}

function hudUrl(assets,key){
 // The loader exposes margin-cropped HUD images; the full original is a valid
 // fallback when an older cropped manifest lacks one of the resource pictures.
 const value=assets?.hud?.[key]||assets?.manifest?.hud?.[key];
 const url=typeof value==='string'?value:value?.url;
 return url?new URL(url,import.meta.url).href:null;
}

export function createEconomyBar(originalAssets,referenceMode){
 if(!referenceMode||typeof document==='undefined')return NOOP;
 const top=document.getElementById('top');if(!top)return NOOP;
 if(bars.has(top))return bars.get(top);
 installStyle();top.classList.add('ee2-economy-top');document.documentElement.classList.add('ee2-economy');
 const before=[...top.childNodes],obsolete=new Set(),cells=[],counters=[],labels=[];
 const first=document.createElement('div');first.className='ee2-economy-first';
 const resources=document.createElement('div');resources.className='ee2-economy-resources';resources.setAttribute('role','group');resources.setAttribute('aria-label','Rohstoffvorräte');
 const summary=document.createElement('div');summary.className='ee2-economy-summary';
 const controls=document.createElement('div');controls.className='ee2-economy-controls';controls.setAttribute('role','group');controls.setAttribute('aria-label','Spielsteuerung');
 for(const slot of DISPLAY_ORDER){
  const existing=document.getElementById('r'+slot),counter=existing||document.createElement('b');
  if(existing&&existing.parentElement!==top)obsolete.add(existing.parentElement);
  counter.id='r'+slot;counter.textContent=counter.textContent||'0';
  const cell=document.createElement('span');cell.className='r ee2-economy-resource';cell.dataset.resource=RESOURCE_KEYS[slot];
  const imageUrl=hudUrl(originalAssets,'icon_resource_'+RESOURCE_KEYS[slot]);
  if(imageUrl){const image=document.createElement('img');image.className='resource-icon';image.src=imageUrl;image.alt='';image.draggable=false;cell.append(image);}
  const label=document.createElement('span');label.className='ee2-economy-name';label.textContent=RESOURCE_NAMES[slot];
  cell.title=RESOURCE_HELP[slot];cell.append(label,counter);resources.append(cell);
  cells[slot]=cell;counters[slot]=counter;labels[slot]=label;
 }
 for(const old of obsolete)if(top.contains(old))old.remove();
 for(const id of ['pop','epoch','science']){
  const node=document.getElementById(id);if(!node||!top.contains(node))continue;
  const wrapper=id==='science'?node:node.parentElement===top?node:node.parentElement;
  if(id==='epoch')wrapper.classList.add('ee2-economy-epoch');
  if(id==='pop')wrapper.title='Bevölkerung / Bevölkerungslimit';
  if(id==='science')wrapper.title='Technologiepunkte. Bürger in Universitäten erzeugen Forschungspunkte.';
  summary.append(wrapper);
 }
 for(const node of before)if(node.parentNode===top&&(node.nodeType!==3||node.textContent.trim()))controls.append(node);
 first.append(resources,summary);top.replaceChildren(first,controls);
 const idle=document.getElementById('idle');if(idle)idle.title='Nächsten freien Bürger auswählen (F)';
 const territory=document.getElementById('terr');if(territory?.parentElement)territory.parentElement.title='Kontrollierte Gebiete. Ein fertiges Stadtzentrum genügt bei angrenzendem Land; abgelegen ist zusätzlich eine fertige Festung nötig.';
 const number=new Intl.NumberFormat('de-DE',{maximumFractionDigits:0});
 const api={update(ui={}){
  const active=ui.resourceActive?.length===9?ui.resourceActive:epochResources(ui.epoch);
  for(let slot=0;slot<9;slot++){
   cells[slot].hidden=!active[slot];
   const value=ui.res?.[slot];if(value!==undefined)counters[slot].textContent=Number.isFinite(+value)?number.format(+value):String(value);
   const name=ui.resourceNames?.[slot]||RESOURCE_NAMES[slot];
   if(labels[slot].textContent!==name)labels[slot].textContent=name;
   cells[slot].setAttribute('aria-label',name+': '+counters[slot].textContent);
  }
 }};
 bars.set(top,api);api.update();return api;
}
