// OpenEpoch checkpoint carrier. Its payload is independent of WebRTC or ENet.
import {storeF32} from './source-solo-clock.js?v=native44';
import {groupedOwnerLayout} from './source-network-round.js?v=native54';

const MAGIC = 0x31524e45; // ENR1
const SHARED_OWNER_MAGIC = 0x32524e45; // ENR2, two controllers with one canonical owner
const GROUPED_OWNER_MAGIC = 0x33524e45; // ENR3, an admitted controller map with persistent world owners
const HEADER = 20;
const SHA_BYTES = 32;
const MAX_WORLD_BYTES = 16 * 1024 * 1024;
const integer = (value, min, max) => Number.isInteger(value) && value >= min && value <= max;

function groupedMetadata(layout) {
  const humanSeats=(layout>>>8)&255, groupedWorldHumans=layout>>>16;
  if (!integer(humanSeats,2,4)) throw Error('Invalid grouped checkpoint layout');
  const groupedOwners=Array.from({length:humanSeats},(_,seat)=>(layout>>>(seat*2))&3);
  const packed=groupedOwnerLayout(groupedOwners,humanSeats,groupedWorldHumans);
  if (!packed || ((groupedWorldHumans<<16)|packed)!==layout) throw Error('Invalid grouped checkpoint layout');
  return {humanSeats,groupedWorldHumans,groupedOwners:Object.freeze(groupedOwners)};
}

function valid({humanSeats, tick, rate, requested, locked, sharedOwner}) {
  return integer(humanSeats, 2, 4) && (sharedOwner!==true || humanSeats===2) &&
    integer(tick, 1, 0xfffffffe) && integer(rate, 5, 30) &&
    Number.isFinite(requested) && requested >= Math.fround(0.05) &&
    requested <= Math.min(Math.fround(1.6),storeF32(rate * Math.fround(0.1))) &&
    typeof locked === 'boolean';
}

async function digest(bytes) {
  return new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
}

/** Called at the common world-save record position inside the admitted aggregate. */
export async function captureNetworkCheckpoint(peer) {
  if (peer.round < 1 || peer.within !== 0 || peer.terminal || peer.alone || peer.desync)
    throw Error('Network save requires an admitted command boundary');
  const {wasm, rate, requested, locked, sharedOwner} = peer;
  const layout=peer.groupedOwners?wasm.grouped_resume_layout():0;
  const grouped=peer.groupedOwners?groupedMetadata(layout):null;
  const humanSeats=grouped?.humanSeats??peer.humanSeats;
  const tick = wasm.game_tick();
  if (!valid({humanSeats, tick, rate, requested, locked, sharedOwner})) throw Error('Invalid checkpoint settings');
  const at = grouped ? wasm.save_grouped_resume() : sharedOwner ? wasm.save_shared_owner_resume() : wasm.save_network_resume();
  const length = wasm.save_len();
  if (!integer(length, 1, MAX_WORLD_BYTES)) throw Error('Network world cannot be saved');
  const bytes = new Uint8Array(HEADER + length + SHA_BYTES);
  const view = new DataView(bytes.buffer);
  view.setUint32(0, grouped ? GROUPED_OWNER_MAGIC : sharedOwner ? SHARED_OWNER_MAGIC : MAGIC, true);
  view.setUint32(4, length, true);
  view.setUint32(8, tick, true);
  view.setFloat32(12, requested, true);
  bytes[16] = humanSeats;
  bytes[17] = rate;
  bytes[18] = Number(locked) | ((grouped?.groupedWorldHumans??0)<<1);
  bytes[19] = layout&255;
  bytes.set(new Uint8Array(wasm.memory.buffer, at, length), HEADER);
  bytes.set(await digest(bytes.subarray(0, HEADER + length)), HEADER + length);
  return bytes;
}

/** Validate the exact shared file before a local transport seat is rebound. */
export async function readNetworkCheckpoint(input) {
  if (!(input instanceof Uint8Array) || !integer(input.length, HEADER + SHA_BYTES + 1, HEADER + SHA_BYTES + MAX_WORLD_BYTES))
    throw Error('Invalid checkpoint length');
  const bytes = input.slice();
  const view = new DataView(bytes.buffer);
  const magic = view.getUint32(0, true);
  const length = view.getUint32(4, true);
  const grouped=magic===GROUPED_OWNER_MAGIC;
  if ((!grouped&&magic!==MAGIC&&magic!==SHARED_OWNER_MAGIC) || (!grouped&&(bytes[19]!==0||bytes[18]>1)) ||
      length !== bytes.length - HEADER - SHA_BYTES)
    throw Error('Invalid checkpoint header');
  const meta = {humanSeats:bytes[16], tick:view.getUint32(8, true), rate:bytes[17],
    requested:view.getFloat32(12, true), locked:(bytes[18]&1)===1,
    ...(magic===SHARED_OWNER_MAGIC?{sharedOwner:true}:grouped?
      groupedMetadata(((bytes[18]>>1)<<16)|(bytes[16]<<8)|bytes[19]):{})};
  if (!valid(meta)) throw Error('Invalid checkpoint settings');
  const end = HEADER + length;
  const sha = await digest(bytes.subarray(0, end));
  if (!sha.every((value, index) => bytes[end + index] === value)) throw Error('Checkpoint digest mismatch');
  return {meta:Object.freeze(meta), world:bytes.slice(HEADER, end), sha};
}

export async function restoreNetworkCheckpoint(wasm, input, authenticatedSeat, expectedRoster, advertisedDigest) {
  const decoded = await readNetworkCheckpoint(input);
  const {sha} = decoded;
  if (!(advertisedDigest instanceof Uint8Array) || advertisedDigest.length !== SHA_BYTES ||
      !sha.every((value, index) => value === advertisedDigest[index]))
    throw Error('Checkpoint does not match the host file');
  applyNetworkCheckpoint(wasm, decoded, authenticatedSeat, expectedRoster);
  return decoded.meta;
}

export function applyNetworkCheckpoint(wasm, {meta, world}, authenticatedSeat, expectedRoster, validateOnly = false) {
  if (meta.humanSeats !== expectedRoster || !integer(authenticatedSeat, 0, expectedRoster - 1))
    throw Error('Checkpoint does not match admitted roster');
  const at = wasm.load_alloc(world.length);
  new Uint8Array(wasm.memory.buffer, at, world.length).set(world);
  if (meta.groupedOwners) {
    const packed=groupedOwnerLayout(meta.groupedOwners,meta.humanSeats,meta.groupedWorldHumans);
    const load=validateOnly?wasm.validate_grouped_resume:wasm.load_grouped_resume;
    if (!packed || typeof load!=='function' ||
        load((meta.groupedWorldHumans<<16)|packed,authenticatedSeat,meta.tick,meta.rate,meta.requested)!==1)
      throw Error('Checkpoint world is incompatible');
    return;
  }
  const load = meta.sharedOwner ? (validateOnly ? wasm.validate_shared_owner_resume : wasm.load_shared_owner_resume)
    : (validateOnly ? wasm.validate_network_resume : wasm.load_network_resume);
  if (typeof load !== 'function' || (meta.sharedOwner
    ? load(authenticatedSeat, meta.tick, meta.rate, meta.requested)
    : load(meta.humanSeats, authenticatedSeat, meta.tick, meta.rate, meta.requested)) !== 1)
    throw Error('Checkpoint world is incompatible');
}
