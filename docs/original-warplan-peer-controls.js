import {originalWarplanControls,originalWarplanLayout} from './original-warplan-layout.js';

export function createWarplanPeerControls({element,button,rect,editor,peer,recipient,clock,refresh}) {
  const ledger=element('div','fullmap-warplan-ledger');
  const controls=Object.fromEntries(['send','sendAllies','accept','reject'].map(key=>[key,element('button','fullmap-warplan-button')]));
  const chooser=element('div','fullmap-warplan-chooser');
  chooser.setAttribute('role','dialog');
  const panel=document.createElement('div'), title=document.createElement('h2');
  const row=document.createElement('label'), check=document.createElement('input'), name=document.createTextNode('');
  const preset=document.createElement('div'), footer=document.createElement('div');
  const all=document.createElement('button'), allies=document.createElement('button'), enemies=document.createElement('button');
  const ok=document.createElement('button'), cancel=document.createElement('button');
  const options=[all,allies,enemies,ok,cancel];
  for(const choice of options)choice.type='button';
  check.type='checkbox';row.append(check,name);preset.append(all,allies,enemies);footer.append(ok,cancel);
  panel.append(title,row,preset,footer);chooser.append(panel);
  let selected=null, dialog=null, remember=false, ledgerKey='';

  const boundPeer=()=>{const exchange=peer?.();return exchange?.enabled?exchange:null;};
  function ready(){const exchange=boundPeer();return exchange&&!exchange.pending()&&recipient?.()&&editor.hasElements()?exchange:null;}
  const update=()=>{ledgerKey='';refresh();};
  function close(){dialog=null;chooser.hidden=true;update();}
  function send(){
    const exchange=ready();
    if(!exchange)return false;
    if(!exchange.send(editor.snapshot().draft,clock()))return false;
    selected=null;
    close();
    return true;
  }
  controls.send.onclick=()=>{
    if(!ready())return;
    dialog={type:'send'};check.checked=remember;update();
  };
  controls.sendAllies.onclick=()=>{if(recipient?.()?.relation===2)send();};
  controls.accept.onclick=()=>{if(selected?.kind==='received'){dialog={type:'response',id:selected.id,accepted:true};update();}};
  controls.reject.onclick=()=>{if(selected?.kind==='received'){dialog={type:'response',id:selected.id,accepted:false};update();}};
  cancel.onclick=close;
  ok.onclick=()=>{
    if(dialog?.type==='send' && check.checked){remember=true;send();}
    else if(dialog?.type==='response' && boundPeer()?.respond(dialog.id,dialog.accepted))close();
  };
  function selectRecipient(value){check.checked=value;ok.disabled=!value;}
  check.onchange=()=>selectRecipient(check.checked);
  all.onclick=()=>selectRecipient(true);
  allies.onclick=()=>selectRecipient(recipient?.()?.relation===2);
  enemies.onclick=()=>selectRecipient(recipient?.()?.relation===0);

  function layout(art,width,height,epoch,visible){
    const exchange=boundPeer();
    if(!visible || !exchange){
      ledger.hidden=chooser.hidden=true;
      for(const node of Object.values(controls))node.hidden=true;
      return;
    }
    const {sent,received,pending}=exchange.snapshot();
    const planned=selected && (selected.kind==='sent'?sent:received).find(record=>record.id===selected.id);
    const accepting=selected?.kind==='received'&&planned?.status===1;
    const target=recipient?.();
    const supported={send:!selected,sendAllies:!selected,accept:accepting,reject:accepting};
    const positions=originalWarplanLayout(art,width,height), bindings=originalWarplanControls(art,epoch);
    for(const [key,node] of Object.entries(controls)){
      node.hidden=!supported[key];
      if(node.hidden)continue;
      const binding=bindings[key];
      button(node,positions.get(binding.name),binding.label,binding.sprites,false);
      node.disabled=key==='send'||key==='sendAllies' ? !!pending||!target||!editor.hasElements()||key==='sendAllies'&&target.relation!==2 : false;
    }
    const ledgerRect=positions.get('fullmap_warplan_players_cont');
    rect(ledger,{...ledgerRect,height:Math.max(1,positions.get(bindings.send.name).y-ledgerRect.y-8)});
    ledger.style.backgroundImage=`url("${art.role('panelInset',epoch)?.url}")`;
    ledger.hidden=false;
    const key=JSON.stringify([selected,sent.map(p=>[p.id,p.status]),received.map(p=>[p.id,p.status])]);
    if(key!==ledgerKey){
      ledgerKey=key;ledger.replaceChildren();
      const choice=(label,record)=>{
        const item=document.createElement('button');
        item.type='button';item.textContent=label;item.setAttribute('aria-pressed',String(JSON.stringify(selected)===JSON.stringify(record)));
        item.onclick=()=>{selected=record;update();};ledger.append(item);
      };
      choice('N. Kriegsplan',null);
      for(const [kind,records] of [['sent',sent],['received',received]])for(const record of records){
        const time=`${Math.floor(record.timestamp/60)}:${String(record.timestamp%60).padStart(2,'0')}`;
        const state=record.status===2?' · angenommen':record.status===3?' · abgelehnt':'';
        choice(`${kind==='sent'?'Geschickt':'Erhalten'} ${time}${state}`,{kind,id:record.id});
      }
    }
    chooser.hidden=!dialog;
    if(!dialog)return;
    const label=key=>art.source.labels[`tx_ig_warplans_${key}`]?.text ?? '';
    const choosing=dialog.type==='send';
    title.textContent=choosing?label('sendwarplan'):dialog.accepted?'Diesen Kriegsplan annehmen':'Diesen Kriegsplan ablehnen';
    chooser.setAttribute('aria-label',title.textContent);
    row.hidden=preset.hidden=!choosing;
    name.data=target?.name ?? '';
    all.textContent=label('all');allies.textContent=label('allies');enemies.textContent=label('enemies');
    ok.textContent=choosing?label('sendcaps'):'OK';cancel.textContent=label('cancel');
    ok.disabled=choosing&&(!target||!check.checked);
  }

  function key(event){
    if(dialog && event.key==='Escape'){
      event.preventDefault();if(!event.repeat)close();return true;
    }
    if(dialog || !boundPeer() || selected || event.key.toLowerCase()!=='s' || !event.ctrlKey || event.altKey || event.metaKey) return false;
    event.preventDefault();
    if(!event.repeat)(event.shiftKey?controls.sendAllies:controls.send).onclick();
    return true;
  }

  function visiblePaths(){
    if(!selected)return editor.snapshot();
    const records=boundPeer()?.snapshot()[selected.kind] ?? [];
    const plan=records.find(record=>record.id===selected.id);
    return {draft:plan?.paths ?? null,drawing:false};
  }

  return Object.freeze({layout,key,visiblePaths,editing:()=>!dialog&&!selected&&!boundPeer()?.pending(),
    modal:()=>!!dialog,reset:()=>{selected=null;remember=false;close();},close});
}
