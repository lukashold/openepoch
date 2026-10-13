// Multiplayer: deterministic lockstep over a WebRTC data channel.
//
// Only player commands travel over the wire, never game state. Both browsers run
// the same WebAssembly simulation and apply the same commands on the same tick,
// so a 4 000-unit battle costs the same few bytes per second as an empty map.
//
// There is no server: the two browsers exchange one invite code and one reply
// code by hand (chat, mail, ...) and then talk peer to peer.

export const TURN_TICKS = 2;   // one network turn = 2 simulation ticks = 100 ms
export const DELAY = 2;        // commands run this many turns after they are issued

/** Turn-based command scheduler. `send(ArrayBuffer)` delivers to the peer. */
export class Lockstep {
  constructor(wasm, local, send) {
    this.wasm = wasm; this.local = local; this.peer = 1 - local; this.send = send;
    this.turn = 0; this.tickIn = 0; this.sent = DELAY - 1;
    this.cmds = [new Map(), new Map()];     // per seat: turn -> Uint8Array of commands
    this.myHash = new Map(); this.peerHash = new Map();
    this.desync = false; this.waiting = false; this.alone = false;
    const empty = new Uint8Array(0);
    for (let t = 0; t < DELAY; t++) { this.cmds[0].set(t, empty); this.cmds[1].set(t, empty); }
  }

  /** Advance one simulation tick if the peer's input for it has arrived. */
  step() {
    const w = this.wasm;
    if (this.tickIn === 0) {
      // publish what we did since the last turn; it takes effect DELAY turns from now
      if (this.sent < this.turn + DELAY) {
        const at = this.turn + DELAY;
        const n = w.outbox_len(), ptr = w.outbox_ptr();
        const bytes = new Uint8Array(w.memory.buffer, ptr, n).slice();
        w.outbox_clear();
        const hash = w.state_hash() >>> 0; // wasm hands back a signed i32, the wire carries unsigned
        this.cmds[this.local].set(at, bytes);
        this.checkHash(this.turn, hash, this.myHash, this.peerHash);
        const msg = new Uint8Array(12 + n), dv = new DataView(msg.buffer);
        dv.setUint32(0, at, true); dv.setUint32(4, this.turn, true); dv.setUint32(8, hash, true);
        msg.set(bytes, 12);
        if (!this.alone) this.send(msg.buffer);
        this.sent = at;
      }
      if (!this.alone && !this.cmds[this.peer].has(this.turn)) { this.waiting = true; return false; }
      this.waiting = false;
      // seat order, so both sides apply in the same sequence
      for (const seat of [0, 1]) {
        const bytes = this.cmds[seat].get(this.turn);
        this.cmds[seat].delete(this.turn);
        if (bytes && bytes.length) {
          const ptr = w.inbox_alloc(bytes.length); // before touching memory.buffer: this may grow it
          new Uint8Array(w.memory.buffer, ptr, bytes.length).set(bytes);
          w.net_apply(seat);
        }
      }
    }
    w.tick();
    if (++this.tickIn === TURN_TICKS) { this.tickIn = 0; this.turn++; }
    return true;
  }

  checkHash(turn, hash, mine, theirs) {
    if (theirs.has(turn)) { if (theirs.get(turn) !== hash) this.desync = true; theirs.delete(turn); }
    else mine.set(turn, hash);
    if (mine.size > 400) mine.delete(mine.keys().next().value);
  }

  /** Feed a message received from the peer. */
  receive(buf) {
    const dv = new DataView(buf);
    this.cmds[this.peer].set(dv.getUint32(0, true), new Uint8Array(buf, 12).slice());
    this.checkHash(dv.getUint32(4, true), dv.getUint32(8, true), this.peerHash, this.myHash);
  }

  /** The peer is gone: keep playing without waiting for them. */
  dropPeer() { this.alone = true; this.waiting = false; }
}

// ---------------------------------------------------------------- WebRTC with hand-carried signalling

const ICE = { iceServers: [{ urls: 'stun:stun.l.google.com:19302' }] };
const encode = d => btoa(JSON.stringify(d)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
const decode = s => JSON.parse(atob(s.trim().replace(/-/g, '+').replace(/_/g, '/')));

function gathered(pc) {
  return new Promise(resolve => {
    if (pc.iceGatheringState === 'complete') return resolve();
    const done = () => { if (pc.iceGatheringState === 'complete') { pc.removeEventListener('icegatheringstatechange', done); resolve(); } };
    pc.addEventListener('icegatheringstatechange', done);
    setTimeout(resolve, 4000); // good enough: whatever candidates we have by now
  });
}

function wire(pc, ch, on) {
  ch.binaryType = 'arraybuffer';
  ch.onopen = () => on.open(ch);
  ch.onclose = () => on.close();
  ch.onmessage = e => on.message(e.data);
  pc.onconnectionstatechange = () => { if (['failed', 'disconnected', 'closed'].includes(pc.connectionState)) on.close(); };
}

/** Host: returns { code, accept(replyCode) }. */
export async function host(on) {
  const pc = new RTCPeerConnection(ICE);
  wire(pc, pc.createDataChannel('game', { ordered: true }), on);
  await pc.setLocalDescription(await pc.createOffer());
  await gathered(pc);
  return { pc, code: encode(pc.localDescription), accept: reply => pc.setRemoteDescription(decode(reply)) };
}

/** Guest: takes the host's invite code, returns the reply code to send back. */
export async function join(invite, on) {
  const pc = new RTCPeerConnection(ICE);
  pc.ondatachannel = e => wire(pc, e.channel, on);
  await pc.setRemoteDescription(decode(invite));
  await pc.setLocalDescription(await pc.createAnswer());
  await gathered(pc);
  return { pc, code: encode(pc.localDescription) };
}
