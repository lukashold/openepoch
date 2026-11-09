import {addF64, filterSourceCpu, precedingF64, selectAdaptiveTempo, sourceStepDelta, sourceWallInterval, storeF32} from './source-solo-clock.js?v=native44';

const BOOTSTRAP_RATE = 30;
const BOOTSTRAP_LENGTH = 10;
export const SOURCE_ROUND_VERSION = 1;
export const MAX_ROUND_PACKET_BYTES = 0x80000;
const f32 = Math.fround;
const MIN_SPEED = f32(.05);
const MAX_SPEED = f32(1.6);
const MIN_RATE = 5;
const MAX_LENGTH = 46;
const MAX_COMMAND_BYTES = 0x10000;
const MAX_RECORDS = 2048;

function split64(value) {
  const expanded = 134217729 * value;
  const high = expanded - (expanded - value);
  return [high, value - high];
}

function mulF64(left, right) {
  const nearest = left * right;
  const [leftHigh, leftLow] = split64(left);
  const [rightHigh, rightLow] = split64(right);
  const error = leftLow * rightLow - (((nearest - leftHigh * rightHigh)
    - leftLow * rightHigh) - leftHigh * rightLow);
  return error < 0 ? precedingF64(nearest) : nearest;
}

function integerIn(value, min, max) {
  return Number.isInteger(value) && value >= min && value <= max;
}

export function groupedOwnerLayout(owners, controllers, worldHumans = null) {
  const restored=worldHumans!==null;
  if (!integerIn(controllers, restored?2:3, 4) || !Array.isArray(owners) || owners.length !== controllers ||
      (restored&&(!integerIn(worldHumans,2,3)||owners[0]!==0))) return 0;
  let nextOwner = 0, packed = 0;
  for (const [controller, owner] of owners.entries()) {
    if (!integerIn(owner, 0, restored?worldHumans-1:nextOwner)) return 0;
    if (owner === nextOwner) nextOwner++;
    packed |= owner << (controller * 2);
  }
  return restored || (nextOwner >= 2 && nextOwner < controllers) ? (controllers << 8) | packed : 0;
}

function validSpeed(value, cap = MAX_SPEED) {
  return f32(value) === value && value >= MIN_SPEED && value <= cap;
}

export function validSimRate(rate, scale) {
  return integerIn(rate, MIN_RATE, BOOTSTRAP_RATE) &&
    validSpeed(scale, Math.min(MAX_SPEED, storeF32(rate * f32(.1))));
}

export function hostDecision(rate, length, requested, samples, adjust = true, bypass = false) {
  let maxPing = 0, minRender = 10000, maxCPU = 0;
  for (const {ping, render, cpu} of samples) {
    maxPing = Math.max(maxPing, ping, render);
    minRender = Math.min(minRender, render);
    maxCPU = Math.max(maxCPU, cpu);
  }
  const tempo = bypass ? {rate} : adjust ? selectAdaptiveTempo(rate, maxCPU, 1000 / minRender, requested)
    : {rate: BOOTSTRAP_RATE, scale: requested};
  const seconds = mulF64(Math.min(maxPing, 1000) * 1.5, .001);
  const target = Math.trunc(addF64(mulF64(seconds, tempo.rate), 1));
  const nextLength = Math.trunc((length + target) / 2);
  return [
    ...(bypass ? [] : [{type: 'simRate', ...tempo}]),
    {type: 'roundLength', length: nextLength},
  ];
}

function validCommands(bytes) {
  if (!(bytes instanceof Uint8Array) || bytes.length === 0 || bytes.length > MAX_COMMAND_BYTES) return false;
  let at = 0;
  while (at + 14 <= bytes.length) at += 14 + bytes[at + 1] * 4;
  return at === bytes.length;
}

function validRecord(record, humanSeats, checkpoints, grouped) {
  if (!record) return false;
  if (record.type === 'requested') return validSpeed(record.value);
  if (record.type === 'simRate') return validSimRate(record.rate, record.scale);
  if (record.type === 'roundLength') return integerIn(record.length, 1, MAX_LENGTH);
  if (record.type === 'departure') return (grouped || humanSeats > 2) && integerIn(record.seat, 1, humanSeats - 1);
  if (record.type === 'checkpoint') return checkpoints && integerIn(record.id, 1, 0xffffffff);
  return record.type === 'commands' && integerIn(record.seat, 0, humanSeats - 1) && validCommands(record.bytes);
}

function validSample(sample) {
  return !!sample && integerIn(sample.ping, 0, 60000) && integerIn(sample.render, 1, 10000) &&
    f32(sample.cpu) === sample.cpu && sample.cpu >= 0 && sample.cpu <= 30;
}

function copyRecord(record) {
  return record.type === 'commands' ? {...record, bytes: record.bytes.slice()} : {...record};
}

const openRound = round => ({round,records:[],bytes:9});
const framedLength = record => 5+(record.type==='commands'?record.bytes.length+1:
  record.type==='simRate'?8:record.type==='departure'?1:4);

/** An original-style host scheduler over one authenticated link per remote seat. */
export class SourceRoundPeer {
  constructor(wasm, seat, send, {renderFps, linkMs, nowSeconds, onRequested, requested = 1, locked = true, humanSeats = 2, resume = null, onCheckpoint = null, sharedOwner = false, groupedOwners = null, onAdmittedStep = null}) {
    const tick = wasm.game_tick();
    const groupedLayout = groupedOwners === null ? 0 : groupedOwnerLayout(groupedOwners, humanSeats, resume?.groupedWorldHumans??null);
    const validStart = resume === null ? tick === 0 :
      integerIn(resume.tick, 1, 0xfffffffe) && resume.tick === tick && integerIn(resume.rate, 5, BOOTSTRAP_RATE) &&
      (resume.sharedOwner===true) === sharedOwner && !!resume.groupedOwners===!!groupedOwners && wasm.network_resume_pending?.() === 1;
    if (!integerIn(humanSeats, 2, 4) || !integerIn(seat, 0, humanSeats - 1) ||
        sharedOwner !== (wasm.network_shared_owner?.() === 1) ||
        (sharedOwner && (humanSeats !== 2 || (onCheckpoint !== null && typeof wasm.save_shared_owner_resume !== 'function'))) ||
        (wasm.network_grouped_coop_layout?.() ?? 0) !== groupedLayout ||
        (groupedOwners !== null && (!groupedLayout || sharedOwner ||
          (onCheckpoint !== null && typeof wasm.save_grouped_resume !== 'function'))) ||
        (humanSeats > 2 && (typeof wasm.init_network_roster !== 'function' ||
          typeof wasm.network_roster_peer_disconnected !== 'function')) ||
        !validSpeed(requested) || typeof locked !== 'boolean' ||
        typeof wasm.tick_with_network_source_slot !== 'function' || !validStart) {
      throw new Error('Invalid source-round startup');
    }
    this.wasm = wasm;
    this.local = seat;
    this.humanSeats = humanSeats;
    this.sharedOwner = sharedOwner;
    this.groupedOwners = groupedOwners === null ? null : Object.freeze(groupedOwners.slice());
    this.send = send;
    this.renderFps = renderFps;
    this.linkMs = linkMs;
    this.nowSeconds = nowSeconds;
    this.onRequested = onRequested;
    this.onCheckpoint = onCheckpoint;
    this.onAdmittedStep = onAdmittedStep;
    this.locked = locked;
    this.rate = resume?.rate ?? BOOTSTRAP_RATE;
    this.requested = requested;
    this.effective = requested;
    this.round = -1;
    this.within = 0;
    this.length = BOOTSTRAP_LENGTH;
    this.remainder = 0;
    this.ring = new Array(3);
    this.pending = [];
    this.open = openRound(1);
    this.links = new Map();
    const others = seat===0 ? Array.from({length:humanSeats-1},(_,index)=>index+1) : [0];
    for (const other of others) this.links.set(other,{ack:-3,sample:null,localHashes:new Map(),remoteHashes:new Map()});
    this.cpuSeconds = 0;
    this.waiting = true;
    this.transportAvailable = true;
    this.alone = false;
    this.terminal = false;
    this.desync = false;
    this.failure = '';
  }

  transmit(kind, fields) {
    if (!this.alone) this.send({kind, ...fields});
  }

  receive(sender, message) {
    const link = this.links.get(sender);
    if (!link || !message || this.alone) return false;
    if (message.kind === 'abort') {
      if (this.humanSeats > 2 && this.local === 0) {
        this.abort(`Mitspieler ${sender + 1} hat die Synchronisierung beendet.`);
        return true;
      }
      this.desync = true;
      this.waiting = false;
      return true;
    }
    if (this.desync || this.terminal) return true;
    if (message.kind === 'round' && sender === 0) {
      if (!this.stage(message)) return false;
      this.checkRoundHash(link, message.round - 2, message.hash, false);
      return true;
    }
    if (message.kind === 'ack' && this.local === 0) {
      const sample = {...message.sample, ping:0};
      if (!integerIn(message.round, link.ack + 1, this.round) || !validSample(sample)) return false;
      link.ack = message.round;
      link.sample = sample;
      this.checkRoundHash(link, message.round, message.hash, false);
      if (this.round === -1 && !this.desync) this.activate();
      return true;
    }
    if (message.kind === 'commands' && this.local === 0 && validCommands(message.bytes)) {
      return this.appendOpen([{type:'commands',seat:sender,bytes:message.bytes.slice()}]);
    }
    return false;
  }

  checkRoundHash(link, round, hash, local) {
    const own = local ? link.localHashes : link.remoteHashes;
    const other = local ? link.remoteHashes : link.localHashes;
    if (other.has(round)) {
      if (other.get(round) !== hash) this.abort(`Runde ${round}: Prüfsummen stimmen nicht überein (${other.get(round)} / ${hash}).`);
      other.delete(round);
    } else {
      own.set(round, hash);
      if (own.size > 16) own.delete(own.keys().next().value);
    }
  }

  appendOpen(records) {
    const bytes = records.reduce((length,record)=>length+framedLength(record),this.open.bytes);
    if (bytes > MAX_ROUND_PACKET_BYTES || this.open.records.length+records.length > MAX_RECORDS) return false;
    this.open.bytes = bytes;
    this.open.records.push(...records);
    return true;
  }

  stage({round, records}) {
    if (!integerIn(round, this.round + 1, this.round + 2) || !Array.isArray(records) ||
        records.length > MAX_RECORDS || !records.every(record=>validRecord(record,this.humanSeats,!!this.onCheckpoint,!!this.groupedOwners))) return false;
    const slot = round % 3;
    if (this.ring[slot]?.round === round) return false;
    this.ring[slot] = {round, records: records.map(copyRecord)};
    return true;
  }

  /** Native virtual +4 activates round zero before the first world slot. */
  activate() {
    if (this.round !== -1 || !this.acknowledgementsReady(0)) return false;
    this.round = 0;
    this.enterRound();
    this.waiting = false;
    return true;
  }

  enterRound() {
    const hash = this.wasm.state_hash() >>> 0;
    for (const link of this.links.values()) {
      this.checkRoundHash(link, this.round - 1, hash, true);
      if (this.desync) return;
    }
    const fps = this.renderFps();
    const render = fps > 0 ? Math.max(1, Math.trunc(1000 / fps)) : 100;
    const sample = {ping:0, render:Math.min(10000,render), cpu:storeF32(this.cpuSeconds)};
    if (this.local === 0) {
      this.hostSample = sample;
      const sealed = this.open;
      if (!this.stage(sealed)) throw new Error('Invalid host round aggregate');
      this.open = openRound(this.round + 2);
      this.transmit('round', {...sealed, hash});
    } else {
      // 005D8BC1 uses the render measurement only after round zero.
      if (this.round === 0) sample.render = 100;
      this.transmit('ack', {round:this.round - 1, hash, sample});
    }
  }

  acknowledgementsReady(round) {
    if (this.local !== 0) return true;
    for (const {ack} of this.links.values()) if (ack < round - 1) return false;
    return true;
  }

  setTempo(value) {
    if (this.local !== 0 || this.locked || this.alone || this.terminal || this.desync || !validSpeed(value)) return false;
    if (this.pending.length > MAX_RECORDS-2) return false;
    this.collectHostCommands();
    this.pending.push({type: 'requested', value});
    return true;
  }

  requestCheckpoint(id) {
    if (this.local !== 0 || !this.onCheckpoint || this.alone || this.terminal || this.desync || this.round < 0 ||
        (this.sharedOwner && this.links.size!==1) || (this.groupedOwners && (!this.links.size||!this.wasm.grouped_resume_layout())) ||
        !integerIn(id, 1, 0xffffffff) || this.pending.length > MAX_RECORDS-2) return false;
    this.collectHostCommands();
    this.pending.push({type:'checkpoint',id});
    return true;
  }

  takeOutbox() {
    const w = this.wasm;
    const length = w.outbox_len();
    if (!length) return null;
    const bytes = new Uint8Array(w.memory.buffer, w.outbox_ptr(), length).slice();
    if (!validCommands(bytes)) throw new Error('Invalid WASM outbox');
    w.outbox_clear();
    return bytes;
  }

  collectHostCommands() {
    const bytes = this.takeOutbox();
    if (bytes) this.pending.push({type: 'commands', seat: 0, bytes});
  }

  flush() {
    if (this.local === 0) {
      this.collectHostCommands();
      if (!this.appendOpen(this.pending.splice(0))) throw new Error('Source round command limit exceeded');
    } else {
      const bytes = this.takeOutbox();
      if (bytes) this.transmit('commands', {bytes});
    }
  }

  dispatch(records) {
    const w = this.wasm;
    for (const record of records) {
      if (record.type === 'requested') {
        if (!this.locked) {
          this.requested = record.value;
          this.onRequested(this.requested);
        }
      } else if (record.type === 'simRate') {
        this.rate = record.rate;
        this.effective = record.scale;
      } else if (record.type === 'roundLength') {
        this.length = record.length;
      } else if (record.type === 'departure') {
        const detach = this.groupedOwners ? w.network_grouped_controller_disconnected : w.network_roster_peer_disconnected;
        if (detach(record.seat)!==1) throw new Error('Invalid roster departure');
      } else if (record.type === 'checkpoint') {
        this.onCheckpoint(record.id);
      } else {
        const pointer = w.inbox_alloc(record.bytes.length);
        new Uint8Array(w.memory.buffer, pointer, record.bytes.length).set(record.bytes);
        if (this.groupedOwners) {
          if (w.net_apply_grouped_controller(record.seat)!==1) throw new Error('Invalid grouped controller');
        } else w.net_apply(this.sharedOwner ? 0 : record.seat);
      }
    }
  }

  detachPeer(seat) {
    if ((!this.groupedOwners&&this.humanSeats<=2) || this.local !== 0 || this.terminal || this.desync || !this.links.has(seat)) return false;
    if (!this.appendOpen([{type:'departure',seat}])) {
      this.abort('Netzwerk-Runde konnte den Verbindungsabbruch nicht aufnehmen.');
      return false;
    }
    this.links.delete(seat);
    if (this.round < 0) this.activate();
    return true;
  }

  detachSharedController(seat) {
    if (!this.sharedOwner || this.local !== 0 || !this.links.delete(seat)) return false;
    // Removing a controller leaves the shared world's player and admitted orders intact.
    if (this.round < 0 && !this.terminal && !this.desync) this.activate();
    return true;
  }

  dropPeer() {
    if (this.sharedOwner || this.humanSeats !== 2 || this.alone || this.terminal || this.desync) return false;
    this.alone = true;
    this.waiting = false;
    this.remainder = 0;
    if (this.round < 0) this.round = 0;
    this.ring.fill(null);
    this.pending.length = 0;
    this.open.records.length = 0;
    this.wasm.outbox_clear();
    return true;
  }

  abort(reason = 'Ungültige Netzwerkdaten') {
    if (this.desync || this.terminal || this.alone) return;
    this.failure = reason;
    this.desync = true;
    this.waiting = false;
    try { this.transmit('abort', {}); } catch { /* Failed channel cannot carry the notice. */ }
  }

  setTransportAvailable(available) {
    this.transportAvailable = available;
    if (!available) {
      this.remainder = 0;
      this.waiting = true;
    }
  }

  interpolation() {
    return this.terminal ? 1 : this.remainder / sourceWallInterval(this.rate);
  }

  advance(wallSeconds) {
    let steps = 0, gameSeconds = 0;
    // A queued RAF timestamp can predate begin()'s performance.now() sample.
    if (!Number.isFinite(wallSeconds) || wallSeconds < 0) wallSeconds = 0;
    if (this.round < 0 || this.desync || this.terminal || (!this.alone&&!this.transportAvailable)) return {steps,gameSeconds};
    this.remainder = storeF32(addF64(this.remainder, storeF32(wallSeconds)));
    for (;;) {
      const wallRate = this.rate;
      const interval = sourceWallInterval(wallRate);
      if (this.remainder <= interval) return {steps,gameSeconds};
      const boundary = this.within === this.length;
      const nextRound = this.round + 1;
      const packet = this.ring[nextRound % 3];
      if (!this.alone && boundary && (packet?.round !== nextRound ||
          !this.acknowledgementsReady(this.round))) {
        this.waiting = true;
        this.remainder = 0;
        return {steps,gameSeconds};
      }
      this.waiting = false;
      if (boundary) {
        this.round = nextRound;
        this.within = 0;
        if (!this.alone) {
          this.enterRound();
          if (this.desync) return {steps,gameSeconds};
          this.dispatch(packet.records);
        }
      }
      this.within++;
      if (this.alone) {
        const bytes = this.takeOutbox();
        if (bytes) this.dispatch([{type:'commands', seat:this.local, bytes}]);
      } else if (this.local === 0 && this.within === this.length) {
        this.collectHostCommands();
        const peers = [...this.links].map(([seat,{sample}])=>({...sample,ping:this.linkMs(seat)}));
        this.pending.push(...hostDecision(this.rate, this.length, this.requested, [this.hostSample,...peers]));
      }
      if (!this.alone) this.flush();
      const worldTime = this.onAdmittedStep ? this.wasm.sim_elapsed_seconds() : 0;
      const before = this.nowSeconds();
      const result = this.wasm.tick_with_network_source_slot(wallRate, this.rate, this.effective, +boundary);
      if (result === 2) {
        this.terminal = true;
        this.remainder = 0;
        return {steps,gameSeconds};
      }
      if (result !== 1) {
        throw new Error('Admitted world update rejected');
      }
      const after = this.nowSeconds();
      if (after >= before) this.cpuSeconds = filterSourceCpu(this.cpuSeconds,addF64(after,-before));
      this.onAdmittedStep?.(worldTime);
      gameSeconds = storeF32(addF64(gameSeconds,sourceStepDelta(this.effective,interval)));
      this.remainder = storeF32(addF64(this.remainder, -interval));
      steps++;
    }
  }
}
