// Browser carrier for the negotiated source-round mode. DTLS supplies the peer
// identity and SCTP supplies reliable ordering. All sizes and record types are
// local to this adapter.
import {MAX_ROUND_PACKET_BYTES as MAX_PACKET} from './source-network-round.js?v=native53';

const MAGIC = 0x314e5253; // SRN1, carrier version 1
const ROSTER_MAGIC = 0x324e5253; // SRN2, isolated three/four-seat carrier
const CHECKPOINT_MAGIC = 0x334e5253; // SRN3, negotiated coordinated checkpoints
const SHARED_OWNER_MAGIC = 0x344e5253; // SRN4, shared-owner room
const SHARED_CHECKPOINT_MAGIC = 0x354e5253; // SRN5, shared-owner room with checkpoints
const GROUPED_COOP_MAGIC = 0x364e5253; // SRN6, multiple world owners with grouped controllers
const HEADER = 16;
const FRAGMENT = 0x4000;
const KINDS = {ack:1, commands:2, round:3, abort:4};
const RECORDS = {requested:1, simRate:2, roundLength:3, commands:4, departure:5, checkpoint:6};

function view(bytes) {
  return new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
}

function recordLength(record) {
  if (record.type === 'commands') return record.bytes.length + 1;
  if (record.type === 'simRate') return 8;
  if (record.type === 'departure') return 1;
  return 4;
}

function encode(message, roster, checkpoints) {
  let bytes;
  if (message.kind === 'round') {
    const length = message.records.reduce((total,record)=>total+5+recordLength(record),9);
    if (length > MAX_PACKET) throw new Error('Source round exceeds carrier limit');
    bytes = new Uint8Array(length);
    const d = view(bytes);
    d.setInt32(1,message.round,true);
    d.setUint32(5,message.hash,true);
    let at = 9;
    for (const record of message.records) {
      const length = recordLength(record);
      bytes[at] = RECORDS[record.type];
      d.setUint32(at+1,length,true);
      at += 5;
      if (record.type === 'requested') d.setFloat32(at,record.value,true);
      else if (record.type === 'simRate') {
        d.setUint32(at,record.rate,true);
        d.setFloat32(at+4,record.scale,true);
      } else if (record.type === 'roundLength') d.setUint32(at,record.length,true);
      else if (roster && record.type === 'departure') bytes[at]=record.seat;
      else if (checkpoints && record.type === 'checkpoint') d.setUint32(at,record.id,true);
      else {
        bytes[at] = record.seat;
        bytes.set(record.bytes,at+1);
      }
      at += length;
    }
  } else if (message.kind === 'ack') {
    bytes = new Uint8Array(17);
    const d = view(bytes);
    d.setInt32(1,message.round,true);
    d.setUint32(5,message.hash,true);
    d.setUint32(9,message.sample.render,true);
    d.setFloat32(13,message.sample.cpu,true);
  } else if (message.kind === 'commands') {
    bytes = new Uint8Array(1+message.bytes.length);
    bytes.set(message.bytes,1);
  } else if (message.kind === 'abort') {
    bytes = new Uint8Array(1);
  } else throw new Error('Unknown source-round message');
  bytes[0] = KINDS[message.kind];
  return bytes;
}

function decode(bytes, roster, checkpoints) {
  const d = view(bytes);
  switch (bytes[0]) {
    case KINDS.abort: return bytes.length === 1 ? {kind:'abort'} : null;
    case KINDS.commands: return bytes.length > 1 ? {kind:'commands',bytes:bytes.subarray(1)} : null;
    case KINDS.ack: return bytes.length === 17 ? {kind:'ack',round:d.getInt32(1,true),hash:d.getUint32(5,true),
      sample:{render:d.getUint32(9,true),cpu:d.getFloat32(13,true)}} : null;
    case KINDS.round: break;
    default: return null;
  }
  if (bytes.length < 9) return null;
  const records = [];
  for (let at = 9; at < bytes.length;) {
    if (at+5 > bytes.length || records.length >= 2048) return null;
    const tag = bytes[at];
    const length = d.getUint32(at+1,true);
    at += 5;
    const end = at+length;
    if (end > bytes.length) return null;
    if (tag === RECORDS.requested && length === 4) records.push({type:'requested',value:d.getFloat32(at,true)});
    else if (tag === RECORDS.simRate && length === 8) records.push({type:'simRate',rate:d.getUint32(at,true),scale:d.getFloat32(at+4,true)});
    else if (tag === RECORDS.roundLength && length === 4) records.push({type:'roundLength',length:d.getUint32(at,true)});
    else if (tag === RECORDS.commands && length > 1) records.push({type:'commands',seat:bytes[at],bytes:bytes.subarray(at+1,end)});
    else if (roster && tag === RECORDS.departure && length === 1) records.push({type:'departure',seat:bytes[at]});
    else if (checkpoints && tag === RECORDS.checkpoint && length === 4) records.push({type:'checkpoint',id:d.getUint32(at,true)});
    else return null;
    at = end;
  }
  return {kind:'round',round:d.getInt32(1,true),hash:d.getUint32(5,true),records};
}

export function createSourceRoundCarrier(send, dispatch) {
  return createCarrier(send,dispatch,MAGIC);
}

export function createSourceRosterCarrier(send, dispatch) {
  return createCarrier(send,dispatch,ROSTER_MAGIC);
}

export function createSourceCheckpointCarrier(send, dispatch) {
  return createCarrier(send,dispatch,CHECKPOINT_MAGIC);
}

export function createSourceSharedOwnerCarrier(send, dispatch, checkpoints = false) {
  return createCarrier(send,dispatch,checkpoints ? SHARED_CHECKPOINT_MAGIC : SHARED_OWNER_MAGIC);
}

export function createSourceGroupedCoopCarrier(send, dispatch) {
  return createCarrier(send,dispatch,GROUPED_COOP_MAGIC);
}

function createCarrier(send, dispatch, magic) {
  const checkpoints=magic===CHECKPOINT_MAGIC||magic===SHARED_CHECKPOINT_MAGIC;
  const roster=magic===CHECKPOINT_MAGIC||magic===ROSTER_MAGIC||magic===GROUPED_COOP_MAGIC;
  let nextSend = 0, nextReceive = 0, pending = null, received = 0;
  return {
    send(message) {
      const bytes = encode(message,roster,checkpoints);
      if (bytes.length > MAX_PACKET) throw new Error('Source round exceeds carrier limit');
      for (let at = 0; at < bytes.length; at += FRAGMENT) {
        const payload = bytes.subarray(at,at+FRAGMENT);
        const packet = new Uint8Array(HEADER+payload.length), d = view(packet);
        d.setUint32(0,magic,true);
        d.setUint32(4,nextSend,true);
        d.setUint32(8,bytes.length,true);
        d.setUint32(12,at,true);
        packet.set(payload,HEADER);
        send(packet.buffer);
      }
      nextSend = (nextSend+1)>>>0;
    },
    receive(packet) {
      if (!(packet instanceof ArrayBuffer) || packet.byteLength <= HEADER || packet.byteLength > HEADER+FRAGMENT) return false;
      const bytes = new Uint8Array(packet), d = view(bytes);
      if (d.getUint32(0,true) !== magic || d.getUint32(4,true) !== nextReceive || d.getUint32(12,true) !== received) return false;
      const total = d.getUint32(8,true), length = bytes.length-HEADER;
      if (total < 1 || total > MAX_PACKET || length !== Math.min(FRAGMENT,total-received)) return false;
      if (!pending) pending = new Uint8Array(total);
      if (total !== pending.length) return false;
      pending.set(bytes.subarray(HEADER),received);
      received += length;
      if (received < total) return true;
      const message = decode(pending,roster,checkpoints);
      pending = null;
      received = 0;
      nextReceive = (nextReceive+1)>>>0;
      return !!message && dispatch(message);
    },
  };
}

export function selectedRoundTripMilliseconds(stats) {
  if (!stats?.values || !stats.get) return null;
  for (const entry of stats.values()) {
    if (entry.type !== 'transport' || !entry.selectedCandidatePairId) continue;
    const value = stats.get(entry.selectedCandidatePairId)?.currentRoundTripTime;
    if (Number.isFinite(value) && value >= 0) return Math.min(60000,Math.trunc(value*1000));
  }
  return null;
}
