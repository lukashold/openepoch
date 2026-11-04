// Exercise the newly available UP roles through ordinary placement/training APIs.
// No entity memory, save bytes, private game function or renderer instance is edited.
export function createUpProductionShowcase(wasm){
 if(wasm.ruleset()!==2)throw Error('Native UP production showcase requires ruleset2');
 const decoder=new TextDecoder(),readUi=()=>{const p=wasm.ui();return JSON.parse(decoder.decode(new Uint8Array(wasm.memory.buffer,p,wasm.ui_len())));};
 for(const code of [0,2,3])if(wasm.cheat(code)!==1)throw Error('Production showcase cheat setup failed');
 const sx=Math.floor(wasm.start_x()),sz=Math.floor(wasm.start_y()),built=[],trained=[];
 const goals=new Set([28,43]);
 for(const host of [48,55]){
  if(!goals.size)break;
  if(wasm.select_idle_worker()===0xffffffff)throw Error('No actual citizen available for production fixture');
  const action=readUi().act.find(a=>a.k==='b'&&a.id===host);
  if(!action)continue;
  let position;
  for(let radius=4;radius<=28&&!position;radius++)for(let dz=-radius;dz<=radius&&!position;dz++)for(let dx=-radius;dx<=radius;dx++){
   if(Math.max(Math.abs(dx),Math.abs(dz))!==radius)continue;
   const x=sx+dx,z=sz+dz;
   if(wasm.place_error_rotated(host,x,z,0)===0&&wasm.cmd_place(host,x,z)===1){position={x,z};break;}
  }
  if(!position)throw Error('No valid host placement for '+host);
  wasm.tick();
  const center={x:position.x+action.size/2,z:position.z+action.size/2};
  wasm.select_none();wasm.select_point(center.x,center.z,0,0,0,0,256,256);
  const menu=readUi();
  if(menu.sel.n!==1||menu.sel.def!==host||menu.sel.built!==1)throw Error('Actual completed host was not selected: '+JSON.stringify({host,position,center,selected:menu.sel}));
  const available=menu.act.filter(a=>a.k==='t').map(a=>a.id);
  built.push({host,...position,center,size:action.size,trainable:available});
  for(const id of [...goals])if(available.includes(id)){
   if(wasm.cmd_train(id)!==1)throw Error('Actual source-role training rejected: '+id);
   wasm.tick();goals.delete(id);trained.push({id,host,tick:wasm.game_tick()});
  }
 }
 if(goals.size)throw Error('No actual production host offered role(s): '+[...goals]);
 const n=wasm.render(1,0,0,256,256,-1,0,0),instances=new Float32Array(wasm.memory.buffer,wasm.inst_ptr(),n*12);
 const typed=typeof wasm.schema_revision==='function'&&[6,7,8].includes(wasm.schema_revision());
 const spawned=trained.map(({id})=>({id,instances:Array.from({length:n},(_,i)=>i).filter(i=>{const raw=instances[i*12+4];return typed?(wasm.render_asset_kind(raw)>>>0)===0&&(wasm.render_asset_source_id(raw)>>>0)===id:raw===id;}).length}));
 if(spawned.some(row=>!row.instances))throw Error('Trained source role did not reach production render stream');
 return {built,trained,spawned,tick:wasm.game_tick(),
  source:'Actual single-player cheats, idle-citizen selection, placement validation, cmd_place, completed-host selection, production-menu availability, cmd_train, ticks and production render stream.'};
}
