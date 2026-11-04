// Own presentation ABI. Copy and validate the sidecar belonging to one render
// call; never infer an actor identity from its animation seed or position.
export const RENDER_ACTOR_META=Object.freeze({version:1,stride:64,none:0xffffffff});
const API=['render_meta_version','render_meta_stride','render_meta_count','render_meta_len','render_meta_ptr','render_meta_session','render_meta_frame','entity_alive'];
const uint=value=>Number.isInteger(value)&&value>=0&&value<=0xffffffff;
const fail=message=>{throw new Error('Render actor metadata: '+message);};

export function createRenderActorMetadata(wasm){
 const present=API.filter(key=>typeof wasm?.[key]==='function');
 if(!present.length)return null;
 if(present.length!==API.length)fail('incomplete presentation ABI');
 if((wasm.render_meta_version()>>>0)!==1||(wasm.render_meta_stride()>>>0)!==64)fail('unsupported record format');
 if(!wasm.memory?.buffer)fail('missing WASM memory');

 function snapshot({count,instances,namespace}={}){
  if(!uint(count)||count>60000)fail('invalid render instance count');
  if(!(instances instanceof Float32Array)||instances.length!==count*12)fail('instance stream length does not match');
  const version=wasm.render_meta_version()>>>0,stride=wasm.render_meta_stride()>>>0;
  const actualCount=wasm.render_meta_count()>>>0,bytes=wasm.render_meta_len()>>>0,ptr=wasm.render_meta_ptr()>>>0;
  const session=wasm.render_meta_session()>>>0,frame=wasm.render_meta_frame()>>>0;
  if(version!==1||stride!==64||actualCount!==count||bytes!==count*64)fail('sidecar does not belong to this render stream');
  const memory=wasm.memory.buffer;
  if((count===0)!==(ptr===0))fail('empty-frame pointer contract does not match');
  if(ptr%4||ptr>memory.byteLength||bytes>memory.byteLength-ptr)fail('sidecar lies outside WASM memory');
  if(count&&!session)fail('uninitialized world session');
  const copy=new Uint8Array(memory,ptr,bytes).slice(),view=new DataView(copy.buffer);
  const actors=[],rows=Array(count).fill(null),byHandle=new Map();
  for(let index=0;index<count;index++){
   const at=index*64,handle=view.getUint32(at,true);
   if(handle===0xffffffff)continue;
   if(byHandle.has(handle))fail('duplicate main-mesh actor handle');
   const record={index,handle,def:view.getUint32(at+4,true),team:view.getUint32(at+8,true),order:view.getUint32(at+12,true),
    hpRatio:view.getFloat32(at+16,true),built:view.getFloat32(at+20,true),renderX:view.getFloat32(at+24,true),renderY:view.getFloat32(at+28,true),renderAltitude:view.getFloat32(at+32,true),
    simX:view.getFloat32(at+36,true),simY:view.getFloat32(at+40,true),velocityX:view.getFloat32(at+44,true),velocityY:view.getFloat32(at+48,true),
    renderYaw:view.getFloat32(at+52,true),renderTime:view.getFloat32(at+56,true),renderFlags:view.getUint32(at+60,true)};
   for(const key of ['hpRatio','built','renderX','renderY','renderAltitude','simX','simY','velocityX','velocityY','renderYaw','renderTime'])if(!Number.isFinite(record[key]))fail('non-finite actor '+key);
   if(record.hpRatio<0||record.hpRatio>1||record.built<0||record.built>1)fail('invalid normalized actor state');
   const base=index*12;
   for(const [key,offset]of [['renderX',0],['renderY',1],['renderAltitude',2],['renderYaw',3],['team',5],['renderFlags',7],['renderTime',10]]){
    if(!Object.is(record[key],instances[base+offset]))fail('sorted actor field differs from the render instance: '+key);
   }
   if(namespace){
    const asset=namespace.asset(instances[base+4]);
    if(!namespace.wide||asset?.kind!==0||asset.sourceId!==record.def)fail('actor definition does not match its typed render binding');
   }
   Object.freeze(record);rows[index]=record;actors.push(record);byHandle.set(handle,record);
  }
  if((wasm.render_meta_session()>>>0)!==session||(wasm.render_meta_frame()>>>0)!==frame)fail('render snapshot changed while reading');
  Object.freeze(actors);Object.freeze(rows);
  return Object.freeze({version,stride,count,session,frame,actors,
   forInstance:index=>Number.isInteger(index)&&index>=0&&index<count?rows[index]:null,
   forHandle:handle=>uint(handle)?byHandle.get(handle)||null:null});
 }
 function alive(handle){
  if(!uint(handle))throw new RangeError('Actor handle must be an unsigned32-bit integer');
  if(handle===0xffffffff)return false;
  const result=wasm.entity_alive(handle)>>>0;
  if(result>1)fail('invalid entity_alive result');
  return result===1;
 }
 return Object.freeze({version:1,stride:64,snapshot,alive});
}
