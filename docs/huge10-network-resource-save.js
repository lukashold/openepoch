import {readHuge10Checkpoint as readLegacy, isHuge10Digest, MAX_HUGE10_SAVE} from './huge10-network-save.js?v=resume122';

export {isHuge10Digest, MAX_HUGE10_SAVE};

const FILE_MAGIC=0x38524e48; // HNR8
const WORLD_MAGIC=0x384e4750; // PGN8
const LEGACY_MAGIC=0x31524e48; // HNR1
const LEGACY_BUILD='3764334d686c91a8a3245212129179f12365f435fdf0c68b149a5ff045c533bf';
const HEADER=24;
const HASH_BYTES=32;
const SETTINGS_MAX=4096;
const WORLD_MAX=16*1024*1024;
const decoder=new TextDecoder('utf-8',{fatal:true});
const encoder=new TextEncoder();
const hex=bytes=>Array.from(bytes,value=>value.toString(16).padStart(2,'0')).join('');
const digest=async bytes=>new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));

function checkClock(tick,rate,scale) {
  const maximum=Math.min(Math.fround(1.6),rate*Math.fround(.1));
  if(!Number.isInteger(tick)||tick<1||tick>=0xffffffff||!Number.isInteger(rate)||rate<5||rate>30||
    scale!==Math.fround(scale)||scale<Math.fround(.05)||scale>maximum){
    throw Error('Ungültiger Rundentakt im Pangea-Spielstand.');
  }
}

function checkWorld(world) {
  if(world.length<24||world.length>WORLD_MAX||new DataView(world.buffer,world.byteOffset).getUint32(0,true)!==WORLD_MAGIC){
    throw Error('Unbekannter Pangea-Weltspielstand.');
  }
}

async function pack(world,settings,build,meta) {
  if(!isHuge10Digest(build)||build===LEGACY_BUILD)throw Error('Ungültige Pangea-Version.');
  checkClock(meta.tick,meta.rate,meta.scale);
  checkWorld(world);
  const config=encoder.encode(JSON.stringify({build,settings}));
  if(config.length>SETTINGS_MAX)throw Error('Die Pangea-Einstellungen sind zu groß.');
  const end=HEADER+config.length+world.length;
  const bytes=new Uint8Array(end+HASH_BYTES),view=new DataView(bytes.buffer);
  view.setUint32(0,FILE_MAGIC,true);
  view.setUint32(4,config.length,true);
  view.setUint32(8,world.length,true);
  view.setUint32(12,meta.tick,true);
  view.setUint32(16,meta.rate,true);
  view.setFloat32(20,meta.scale,true);
  bytes.set(config,HEADER);
  bytes.set(world,HEADER+config.length);
  const hash=await digest(bytes.subarray(0,end));
  bytes.set(hash,end);
  return {bytes,digest:hex(hash),tick:meta.tick};
}

export async function captureHuge10Checkpoint(peer,settings,build) {
  if(peer.humanSeats!==2||peer.round<1||peer.within!==0||peer.terminal||peer.alone||peer.desync){
    throw Error('Pangea kann nur an einer bestätigten Rundengrenze gespeichert werden.');
  }
  const w=peer.wasm;
  const meta={tick:w.game_tick(),rate:w.sim_adaptive_rate(),scale:w.sim_adaptive_scale()};
  const address=w.save_network_pangea_huge10_retirement_resume(),length=w.save_len();
  return pack(new Uint8Array(w.memory.buffer,address,length),settings,build,meta);
}

// The peer transfer always uses this reader and never accepts a legacy envelope.
export async function readHuge10Checkpoint(input,build,checkSettings) {
  if(!(input instanceof Uint8Array)||input.length<HEADER+HASH_BYTES+24||input.length>MAX_HUGE10_SAVE){
    throw Error('Ungültige Größe des Pangea-Spielstands.');
  }
  const bytes=input.slice(),view=new DataView(bytes.buffer),end=bytes.length-HASH_BYTES;
  const configLength=view.getUint32(4,true),worldLength=view.getUint32(8,true);
  const meta={tick:view.getUint32(12,true),rate:view.getUint32(16,true),scale:view.getFloat32(20,true)};
  if(view.getUint32(0,true)!==FILE_MAGIC||configLength<2||configLength>SETTINGS_MAX||
    worldLength>WORLD_MAX||HEADER+configLength+worldLength!==end){
    throw Error('Unbekanntes Pangea-Spielstandsformat.');
  }
  checkClock(meta.tick,meta.rate,meta.scale);
  const hash=hex(await digest(bytes.subarray(0,end)));
  if(hash!==hex(bytes.subarray(end)))throw Error('Die Prüfsumme des Pangea-Spielstands stimmt nicht.');
  const json=decoder.decode(bytes.subarray(HEADER,HEADER+configLength)),data=JSON.parse(json);
  if(!isHuge10Digest(build)||build===LEGACY_BUILD||data?.build!==build)throw Error('Der Spielstand gehört zu einer anderen Pangea-Version.');
  const settings=checkSettings(data.settings);
  if(JSON.stringify({build,settings})!==json)throw Error('Die Pangea-Einstellungen im Spielstand sind ungültig.');
  const world=bytes.subarray(HEADER+configLength,end);
  checkWorld(world);
  return {bytes,digest:hash,settings,world,meta};
}

function copyWorld(w,world) {
  const at=w.load_alloc(world.length);
  if(!at)throw Error('Der Pangea-Spielstand ist zu groß.');
  new Uint8Array(w.memory.buffer,at,world.length).set(world);
}

// Migration is a local file action. The WASM validates PGN1 and serializes a
// temporary world in PGN8 without making it the active game.
export async function importHuge10Checkpoint(input,w,build,checkSettings) {
  if(input instanceof Uint8Array&&input.length>=HEADER&&
    new DataView(input.buffer,input.byteOffset).getUint32(0,true)===LEGACY_MAGIC){
    const legacy=await readLegacy(input,LEGACY_BUILD,checkSettings);
    const {tick,rate,scale}=legacy.meta;
    copyWorld(w,legacy.world);
    const address=w.migrate_network_pangea_huge10_retirement_resume(tick,rate,scale,legacy.settings.shuffle);
    const length=w.network_pangea_huge10_retirement_migration_len();
    if(!address||!length)throw Error('Der bisherige Pangea-Spielstand ist inkompatibel.');
    const migrated=await pack(new Uint8Array(w.memory.buffer,address,length),legacy.settings,build,legacy.meta);
    return readHuge10Checkpoint(migrated.bytes,build,checkSettings);
  }
  return readHuge10Checkpoint(input,build,checkSettings);
}

export function applyHuge10Checkpoint(w,archive,owner,validateOnly=false) {
  if(owner!==0&&owner!==1)throw Error('Ungültiger Pangea-Spielerplatz.');
  const {world,meta:{tick,rate,scale},settings:{shuffle}}=archive;
  copyWorld(w,world);
  const apply=validateOnly?w.validate_network_pangea_huge10_retirement_resume:w.load_network_pangea_huge10_retirement_resume;
  if(apply(owner,tick,rate,scale,shuffle)!==1)throw Error('Der Pangea-Weltspielstand ist inkompatibel.');
}
