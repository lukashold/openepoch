const FILE_MAGIC=0x31524e48; // HNR1
const NAVAL_FILE_MAGIC=0x32524e48; // HNR2
const PURSUIT_FILE_MAGIC=0x33524e48; // HNR3
const ANCHORED_FILE_MAGIC=0x34524e48; // HNR4
const EPOCH_STAGE_FILE_MAGIC=0x35524e48; // HNR5
const profiles={
  legacy:{file:FILE_MAGIC,world:0x314e4750,suffix:''},
  naval:{file:NAVAL_FILE_MAGIC,world:0x324e4750,suffix:'_naval'},
  pursuit:{file:PURSUIT_FILE_MAGIC,world:0x334e4750,suffix:'_naval_pursuit',depthTag:0x31445550},
  anchored:{file:ANCHORED_FILE_MAGIC,world:0x344e4750,suffix:'_naval_anchored',depthTag:0x31434e41},
  epochStage:{file:EPOCH_STAGE_FILE_MAGIC,world:0x354e4750,suffix:'_naval_epoch_stage',depthTag:0x3153564e},
};
function archiveProfile(mode){
  if(mode===false)return profiles.legacy;
  if(mode===true)return profiles.naval;
  if(mode==='pursuit')return profiles.pursuit;
  if(mode==='anchored')return profiles.anchored;
  if(mode==='epoch-stage')return profiles.epochStage;
  throw Error('Unbekanntes Pangea-Spielprofil.');
}
const HEADER=24;
const HASH_BYTES=32;
const SETTINGS_MAX=4096;
const WORLD_MAX=16*1024*1024;
export const MAX_HUGE10_SAVE=HEADER+SETTINGS_MAX+WORLD_MAX+HASH_BYTES;
const decoder=new TextDecoder('utf-8',{fatal:true});
const encoder=new TextEncoder();
const hex=bytes=>Array.from(bytes,value=>value.toString(16).padStart(2,'0')).join('');
export const isHuge10Digest=value=>typeof value==='string'&&/^[0-9a-f]{64}$/.test(value);
const digest=async bytes=>new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));

function checkClock(tick,rate,scale) {
  const maximum=Math.min(Math.fround(1.6),rate*Math.fround(.1));
  if(!Number.isInteger(tick)||tick<1||tick>=0xffffffff||!Number.isInteger(rate)||rate<5||rate>30||
    scale!==Math.fround(scale)||scale<Math.fround(.05)||scale>maximum){
    throw Error('Ungültiger Rundentakt im Pangea-Spielstand.');
  }
}

function checkWorld(world,profile) {
  const view=new DataView(world.buffer,world.byteOffset,world.byteLength);
  if(world.length<24||world.length>WORLD_MAX||view.getUint32(0,true)!==profile.world||
    (profile.depthTag&&view.getUint32(8,true)!==profile.depthTag)){
    throw Error('Unbekannter Pangea-Weltspielstand.');
  }
}

export async function captureHuge10Checkpoint(peer,settings,build,mode=false) {
  const profile=archiveProfile(mode);
  if(peer.humanSeats!==2||peer.round<1||peer.within!==0||peer.terminal||peer.alone||peer.desync||!isHuge10Digest(build)){
    throw Error('Pangea kann nur an einer bestätigten Rundengrenze gespeichert werden.');
  }
  const w=peer.wasm;
  const tick=w.game_tick(),rate=w.sim_adaptive_rate(),scale=w.sim_adaptive_scale();
  checkClock(tick,rate,scale);
  const address=w[`save_network_pangea_huge10${profile.suffix}_resume`]();
  const length=w.save_len();
  const world=new Uint8Array(w.memory.buffer,address,length);
  checkWorld(world,profile);
  const config=encoder.encode(JSON.stringify({build,settings}));
  if(config.length>SETTINGS_MAX)throw Error('Die Pangea-Einstellungen sind zu groß.');
  const end=HEADER+config.length+length;
  const bytes=new Uint8Array(end+HASH_BYTES),view=new DataView(bytes.buffer);
  view.setUint32(0,profile.file,true);
  view.setUint32(4,config.length,true);
  view.setUint32(8,length,true);
  view.setUint32(12,tick,true);
  view.setUint32(16,rate,true);
  view.setFloat32(20,scale,true);
  bytes.set(config,HEADER);
  bytes.set(world,HEADER+config.length);
  const hash=await digest(bytes.subarray(0,end));
  bytes.set(hash,end);
  return {bytes,digest:hex(hash),tick};
}

export async function readHuge10Checkpoint(input,build,checkSettings,mode=false) {
  const profile=archiveProfile(mode);
  if(!(input instanceof Uint8Array)||input.length<HEADER+HASH_BYTES+24||input.length>MAX_HUGE10_SAVE){
    throw Error('Ungültige Größe des Pangea-Spielstands.');
  }
  const bytes=input.slice(),view=new DataView(bytes.buffer),end=bytes.length-HASH_BYTES;
  const configLength=view.getUint32(4,true),worldLength=view.getUint32(8,true);
  const tick=view.getUint32(12,true),rate=view.getUint32(16,true),scale=view.getFloat32(20,true);
  if(view.getUint32(0,true)!==profile.file||configLength<2||configLength>SETTINGS_MAX||
    worldLength>WORLD_MAX||HEADER+configLength+worldLength!==end){
    throw Error('Unbekanntes Pangea-Spielstandsformat.');
  }
  checkClock(tick,rate,scale);
  const hash=hex(await digest(bytes.subarray(0,end)));
  if(hash!==hex(bytes.subarray(end)))throw Error('Die Prüfsumme des Pangea-Spielstands stimmt nicht.');
  const json=decoder.decode(bytes.subarray(HEADER,HEADER+configLength)),data=JSON.parse(json);
  if(!isHuge10Digest(build)||data?.build!==build)throw Error('Der Spielstand gehört zu einer anderen Pangea-Version.');
  const settings=checkSettings(data.settings);
  if(JSON.stringify({build,settings})!==json)throw Error('Die Pangea-Einstellungen im Spielstand sind ungültig.');
  const world=bytes.subarray(HEADER+configLength,end);
  checkWorld(world,profile);
  return {bytes,digest:hash,settings,world,meta:{tick,rate,scale}};
}

export function applyHuge10Checkpoint(w,archive,owner,validateOnly=false,mode=false) {
  const profile=archiveProfile(mode);
  if(owner!==0&&owner!==1)throw Error('Ungültiger Pangea-Spielerplatz.');
  const {world,meta:{tick,rate,scale},settings:{shuffle}}=archive;
  const at=w.load_alloc(world.length);
  if(!at)throw Error('Der Pangea-Spielstand ist zu groß.');
  new Uint8Array(w.memory.buffer,at,world.length).set(world);
  const action=validateOnly?'validate':'load';
  const apply=w[`${action}_network_pangea_huge10${profile.suffix}_resume`];
  if(apply(owner,tick,rate,scale,shuffle)!==1)throw Error('Der Pangea-Weltspielstand ist inkompatibel.');
}
