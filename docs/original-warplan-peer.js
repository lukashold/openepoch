import {decodeWarplanPath,encodeWarplanPath,originalWarplanTimestamp,WARPLAN_MAX_BYTES} from './original-warplan-wire.js';

export const WARPLAN_PEER_VERSION = 1;
const SEND = 'warplan-path';
const ACK = 'warplan-path-ack';
const RESPONSE = 'warplan-path-response';
const MAX_BASE64 = Math.ceil(WARPLAN_MAX_BYTES/3)*4;

function packetBytes(value) {
  if (typeof value!=='string' || value.length>MAX_BASE64 || !/^[a-z\d+/]+={0,2}$/i.test(value)) return null;
  try { return Uint8Array.from(atob(value),character=>character.charCodeAt(0)); } catch { return null; }
}

export function createWarplanPeer({local,peerVersion,end,post,recipientAvailable,onChange,onCommit}) {
  if (![0,1].includes(local)) throw new RangeError('War Plans peer seat');
  const own=local+1, peer=2-local;
  const sent=new Map(), received=new Map();
  let pending=null, nextId=1;
  const enabled=peerVersion===WARPLAN_PEER_VERSION;
  const header = (t,id) => ({t,v:WARPLAN_PEER_VERSION,id});

  function emit(packet) {
    try { return post(packet)===true; } catch { return false; }
  }

  function send(paths,seconds) {
    if (!enabled || pending || !recipientAvailable()) return false;
    const timestamp=originalWarplanTimestamp(seconds);
    const payload=encodeWarplanPath({sender:own,recipient:peer,timestamp,paths},end);
    if (!payload) return false;
    const plan=decodeWarplanPath(payload,{sender:own,recipient:peer,end});
    const id=nextId;
    pending={id,plan};
    if (!emit({...header(SEND,id),bytes:btoa(String.fromCharCode(...payload))})) {
      pending=null;
      return false;
    }
    nextId++;
    onChange?.();
    return true;
  }

  function receive(packet) {
    const {t,v,id}=packet ?? {};
    if (!enabled || v!==WARPLAN_PEER_VERSION || !Number.isSafeInteger(id) || id<1) return false;
    if (t===SEND) {
      const bytes=packetBytes(packet.bytes);
      const plan=decodeWarplanPath(bytes,{sender:peer,recipient:own,end});
      if (!plan) return false;
      const previous=received.get(id);
      if (previous) return previous.bytes===packet.bytes && emit(header(ACK,id));
      received.set(id,{id,plan,bytes:packet.bytes});
      emit(header(ACK,id));
      onChange?.();
      return true;
    }
    if (t===ACK && pending?.id===id) {
      sent.set(id,pending);
      pending=null;
      onCommit?.(id);
      onChange?.();
      return true;
    }
    if (t===RESPONSE && typeof packet.accepted==='boolean') {
      const record=sent.get(id);
      if (record?.plan.status!==1) return false;
      record.plan.status=packet.accepted?2:3;
      onChange?.();
      return true;
    }
    return false;
  }

  function respond(id,accepted) {
    const record=received.get(id);
    if (!enabled || record?.plan.status!==1 || typeof accepted!=='boolean' || !emit({...header(RESPONSE,id),accepted})) return false;
    record.plan.status=accepted?2:3;
    onChange?.();
    return true;
  }

  const records = map => [...map.values()].sort((a,b)=>a.plan.timestamp-b.plan.timestamp || a.id-b.id)
    .map(({id,plan})=>({id,...plan}));
  return Object.freeze({send,receive,respond,enabled,
    pending:()=>pending?.id ?? null,
    cancelPending:()=>{if(pending){pending=null;onChange?.();}},
    snapshot:()=>structuredClone({pending:pending?.id ?? null,sent:records(sent),received:records(received)})});
}
