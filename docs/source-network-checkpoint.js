// Portable browser checkpoint and bounded WebRTC file fragments.
import {captureNetworkCheckpoint,readNetworkCheckpoint} from './source-network-resume.js?v=native48';

const FILE_MAGIC=0x31434f45; // EOC1
const CHUNK_MAGIC=0x31415345; // ESA1
const HEADER=8;
const HASH_BYTES=32;
const SETTINGS_BYTES=16384;
export const MAX_CHECKPOINT_BYTES=16*1024*1024+SETTINGS_BYTES+128;
const CHUNK=16384;
export const CHECKPOINT_WINDOW=4*CHUNK;
export const validCheckpointDigest=value=>typeof value==='string'&&/^[0-9a-f]{64}$/.test(value);
export const validCheckpointSize=value=>Number.isInteger(value)&&value>HEADER+HASH_BYTES&&value<=MAX_CHECKPOINT_BYTES;
const encoder=new TextEncoder(),decoder=new TextDecoder('utf-8',{fatal:true});
const digest=async bytes=>new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
const hex=bytes=>Array.from(bytes,value=>value.toString(16).padStart(2,'0')).join('');

export async function captureBrowserCheckpoint(peer,settings) {
  // captureNetworkCheckpoint copies the admitted world synchronously before hashing.
  const tick=peer.wasm.game_tick();
  const inner=await captureNetworkCheckpoint(peer);
  const config=encoder.encode(JSON.stringify(settings));
  if(config.length>SETTINGS_BYTES)throw Error('Die Spieleinstellungen sind zu groß.');
  const bytes=new Uint8Array(HEADER+config.length+inner.length+HASH_BYTES);
  const view=new DataView(bytes.buffer);
  view.setUint32(0,FILE_MAGIC,true);
  view.setUint32(4,config.length,true);
  bytes.set(config,HEADER);
  bytes.set(inner,HEADER+config.length);
  const sha=await digest(bytes.subarray(0,-HASH_BYTES));
  bytes.set(sha,bytes.length-HASH_BYTES);
  return {bytes,digest:hex(sha),tick};
}

export async function readBrowserCheckpoint(bytes,advertised=null) {
  if(!(bytes instanceof Uint8Array)||!validCheckpointSize(bytes.length))throw Error('Ungültige Spielstandsgröße.');
  const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  const configLength=view.getUint32(4,true);
  const end=bytes.length-HASH_BYTES;
  if(view.getUint32(0,true)!==FILE_MAGIC||configLength<2||configLength>SETTINGS_BYTES||HEADER+configLength>=end)
    throw Error('Unbekanntes Mehrspieler-Spielstandsformat.');
  const sha=hex(await digest(bytes.subarray(0,end)));
  if(sha!==hex(bytes.subarray(end))||(advertised!==null&&sha!==advertised))
    throw Error('Die Prüfsumme des Spielstands stimmt nicht.');
  const settings=JSON.parse(decoder.decode(bytes.subarray(HEADER,HEADER+configLength)));
  const decoded=await readNetworkCheckpoint(bytes.subarray(HEADER+configLength,end));
  if(settings?.humanSeats!==decoded.meta.humanSeats)throw Error('Spielstand und Spielerplätze passen nicht zusammen.');
  return {bytes,digest:sha,settings,decoded};
}

export function sendCheckpointWindow(channel,bytes,offset) {
  if(channel.bufferedAmount>2*CHECKPOINT_WINDOW)throw Error('Spielstandübertragung überlastet.');
  const end=Math.min(bytes.length,offset+CHECKPOINT_WINDOW);
  for(;offset<end;){
    const next=Math.min(end,offset+CHUNK);
    const fragment=new Uint8Array(HEADER+next-offset);
    const view=new DataView(fragment.buffer);
    view.setUint32(0,CHUNK_MAGIC,true);
    view.setUint32(4,offset,true);
    fragment.set(bytes.subarray(offset,next),HEADER);
    channel.send(fragment.buffer);
    offset=next;
  }
  return end;
}

export function receiveCheckpointChunk(bytes,offset,packet) {
  if(!(packet instanceof ArrayBuffer)||packet.byteLength!==HEADER+Math.min(CHUNK,bytes.length-offset))
    throw Error('Ungültiger Spielstandabschnitt.');
  const view=new DataView(packet);
  if(offset>=bytes.length||view.getUint32(0,true)!==CHUNK_MAGIC||view.getUint32(4,true)!==offset)
    throw Error('Spielstandabschnitt außerhalb der Reihenfolge.');
  bytes.set(new Uint8Array(packet,HEADER),offset);
  return offset+packet.byteLength-HEADER;
}
