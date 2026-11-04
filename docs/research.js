// Full-tree interface. Costs, availability and command validation belong to WASM.
const EPOCHS=['Steinzeit','Kupferzeit','Bronzezeit','Eisenzeit','Frühes Mittelalter','Mittelalter','Renaissance','Imperiales Zeitalter','Aufklärung','Industriezeitalter','Moderne','Atomzeitalter','Digitales Zeitalter','Gentechnisches Zeitalter','Synthetikzeitalter'];
const BRANCHES=['Wirtschaft','Militär','Reich'];
const HOSTS={'Town Center':'Stadtzentrum','Barracks':'Kaserne','Dock':'Hafen','Airfield':'Flugplatz','University':'Universität','Mill':'Mühle','Stable':'Stall','Market':'Markt','Temple':'Tempel','Warehouse':'Lagerhaus'};
export function researchPanel({wasm,onOrder,onOpen,onClose}) {
 const dialog=document.getElementById('research-dialog'), tabs=document.getElementById('research-epochs'), grid=document.getElementById('research-nodes'), status=document.getElementById('research-status');
 const dec=new TextDecoder();let epoch=0,ui=null,lastSig='';
 const select=document.createElement('select');select.setAttribute('aria-label','Forschungsepoche');
 EPOCHS.forEach((name,i)=>{const o=new Option(`${i+1} · ${name}`,String(i));select.append(o);});tabs.append(select);
 select.onchange=()=>{epoch=+select.value;refresh(ui);};
 function close(){if(!dialog.open)return;dialog.close();onClose();}
 document.getElementById('research-close').onclick=close;
 dialog.addEventListener('cancel',e=>{e.preventDefault();close();});
 function refresh(next){
  ui=next;if(!dialog.open||!ui)return;
  const p=wasm.research_tree(), nodes=JSON.parse(dec.decode(new Uint8Array(wasm.memory.buffer,p,wasm.ui_len())));
  status.textContent=`${ui.research.points.toFixed(1)} Technologiepunkte · +${(ui.research.rate*60).toFixed(1)} / Minute · ${ui.research.count}/6 Technologien für den nächsten Epochenwechsel`;
  const sig=epoch+':'+nodes.map(n=>`${+n.done}${+n.busy}${+n.hostBuilt}${+n.ok}`).join('');
  if(sig===lastSig)return;lastSig=sig;
  grid.replaceChildren();
  for(let b=0;b<3;b++){
   const col=document.createElement('section');const h=document.createElement('h3');h.textContent=BRANCHES[b];col.append(h);
   for(const n of nodes.filter(n=>n.epoch===epoch&&n.branch===b)){
    const button=document.createElement('button');button.type='button';button.className='research-node'+(n.done?' done':'')+(n.busy?' busy':'');
    const name=document.createElement('strong');name.textContent=n.name;
    const cost=document.createElement('span');cost.textContent=`${n.cost} TP · 15 Sek. · ${HOSTS[n.host]||n.host}`;
    const effect=document.createElement('span');effect.textContent=n.effect;
    const state=document.createElement('small');state.textContent=n.done?'✓ Erforscht':n.busy?'Forschung läuft …':epoch>ui.epoch?'Spätere Epoche':!n.hostBuilt?'Benötigt '+(HOSTS[n.host]||n.host):n.ok?'Forschen':ui.research.points<n.cost?'Technologiepunkte fehlen':'Forschungswarteschlange belegt';
    button.append(name,cost,effect,state);button.disabled=!n.ok;
    button.onclick=()=>{if(wasm.cmd_research(n.id))onOrder();refresh(ui);};col.append(button);
   }
   grid.append(col);
  }
 }
 return {refresh,open(next){ui=next;epoch=ui?.epoch||0;lastSig='';select.value=String(epoch);onOpen();dialog.showModal();refresh(ui);}};
}
