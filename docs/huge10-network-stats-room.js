import {host as rtcHost, join as rtcJoin} from './net.js?v=native55';
import {SourceRoundPeer, SOURCE_ROUND_VERSION} from './source-network-round.js?v=native54';
import {createSourceCheckpointCarrier, selectedRoundTripMilliseconds} from './source-network-wire.js?v=native54';
import {CHECKPOINT_WINDOW,receiveCheckpointChunk,sendCheckpointWindow,validCheckpointSize} from './source-network-checkpoint.js?v=native54';
import {HUGE10_SEED,HUGE10_SHUFFLE_STATE} from './huge10-setup.js?v=huge10-browser116';
import {applyHuge10Checkpoint,captureHuge10Checkpoint,isHuge10Digest,MAX_HUGE10_SAVE,readHuge10Checkpoint} from './huge10-network-stats-save.js?v=stats142';

const capability = 'pangea-huge10-native-stats-1';
const prefix = {invite:'OE-H10-I8', answer:'OE-H10-A8'};
const shape = Object.freeze({seed:HUGE10_SEED,shuffle:HUGE10_SHUFFLE_STATE,side:250,humans:2,ai:8,climate:0,victory:'conquest',naval:'torpedo84',pursuit:'move-goal',anchor:'cached-model-1',epochVisual:'source-queue-2',researchQueue:'ordinary-1',typeStats:'retained-1',epoch:10,finalEpoch:14});
const hashWord = value => Number.isInteger(value) && value >= 0 && value <= 0xffffffff;
const exact = (left,right) => JSON.stringify(left) === JSON.stringify(right);
const hex = bytes => Array.from(bytes, byte=>byte.toString(16).padStart(2,'0')).join('');

export const networkShape = shape;

export async function verifyNetworkWasm(response,expected) {
  if (!response.ok) return response;
  const bytes=await response.clone().arrayBuffer();
  const actual=hex(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)));
  if (actual!==expected) throw Error('Die private Pangea-Spieldatei gehört zu einer anderen Version.');
  return response;
}

export function unpackNetworkCode(kind,value) {
  const clean=value.trim();
  const [tag,room,sdp,extra]=clean.split('.');
  if (clean.length>65536 || tag!==prefix[kind] || !/^[0-9a-f]{32}$/.test(room??'') ||
      !/^[A-Za-z0-9_-]+$/.test(sdp??'') || extra!==undefined) {
    throw Error('Dieser Code gehört zu keiner passenden Pangea-Epochenpartie.');
  }
  return {room,sdp};
}

export function mountHuge10NetworkRoom(root,{wasm,build,makeSettings,checkSettings,prepare,launch,renderFps,onAbort,onCheckpoint}) {
  const get=name=>root.querySelector(`[data-h10-room="${name}"]`);
  const input=get('input'),output=get('output'),connect=get('connect'),start=get('start');
  const resumeFile=get('resume'),host=get('host'),guest=get('guest');
  const status=value=>{get('status').textContent=value;};
  let active=null,selectedArchive=null,importToken=0;

  function closeActive() {
    const room=active;
    active=null;
    clearTimeout(room?.save?.timer);
    clearTimeout(room?.transfer?.timer);
    if (room?.peer) room.peer.abort('Der Pangea-Raum wurde geschlossen.');
    room?.ch?.close();
    room?.pc?.close();
    input.value=output.value='';
    connect.disabled=start.disabled=true;
    resumeFile.disabled=false;
  }

  function reset() {
    closeActive();
    importToken++;
    selectedArchive=null;
    resumeFile.value='';
    host.disabled=guest.disabled=false;
    start.textContent='Partie starten';
    status('Einladung für zwei Menschen und acht KI-Gegner erstellen oder einfügen.');
  }

  function fail(room,reason) {
    if (active!==room || room.failed) return;
    room.failed=true;
    clearTimeout(room.save?.timer);
    clearTimeout(room.transfer?.timer);
    room.save=null;
    room.transfer=null;
    connect.disabled=start.disabled=true;
    resumeFile.disabled=false;
    status(reason);
    if (room.peer) {
      room.peer.abort(reason);
      onAbort(reason);
    }
    room.ch?.close();
    room.pc?.close();
  }

  function hello(room,owner,archive=room.archive?.digest??null) {
    return {t:'hello',room:room.id,capability,owner,wasm:build,rounds:SOURCE_ROUND_VERSION,rules:wasm.ruleset(),shape,
      archive};
  }

  function send(room,message) {
    if (room.ch?.readyState!=='open') throw Error('Die Verbindung zum Mitspieler wurde getrennt.');
    room.ch.send(JSON.stringify(message));
  }

  function ready(room) {
    clearTimeout(room.transfer?.timer);
    room.transfer=null;
    room.ready=true;
    start.disabled=room.owner!==0;
    status(room.owner===0?'Beide Spieler sind bereit. Der Gastgeber kann starten.':'Bereit. Der Gastgeber startet die Partie.');
  }

  function acceptHello(room,message) {
    const remote=message?.archive, local=room.archive?.digest??null;
    if((remote!==null&&!isHuge10Digest(remote))||!exact(message,hello(room,1-room.owner,remote))){
      throw Error('Beide Spieler brauchen dieselbe Version der Pangea-Epochenpartie.');
    }
    room.remote=true;
    if(remote===local){ready(room);return;}
    const hostDigest=room.owner===0?local:remote,guestDigest=room.owner===0?remote:local;
    if(!hostDigest||guestDigest!==null){
      const reason='Der gewählte Spielstand stimmt nicht mit der Auswahl des Gastgebers überein.';
      if(room.owner===0)throw Error(reason);
      room.admissionError=reason;
      status(reason);
      return;
    }
    room.transfer={digest:hostDigest,offset:0,timer:setTimeout(()=>{
      fail(room,'Die Übertragung des Pangea-Spielstands hat zu lange gedauert.');
    },120000)};
    if(room.owner===0){
      send(room,{t:'archive-offer',digest:hostDigest,length:room.archive.bytes.length});
      status('Warte auf die Prüfung des Spielstands beim Mitspieler …');
    }else status('Warte auf den Spielstand des Gastgebers …');
  }

  function transferControl(room,message) {
    const transfer=room.transfer;
    if(room.owner===1&&message?.t==='archive-offer'&&!transfer.bytes){
      const {digest,length}=message;
      if(!exact(message,{t:'archive-offer',digest:transfer.digest,length})||
        !validCheckpointSize(length)||length>MAX_HUGE10_SAVE)throw Error('Ungültige Pangea-Spielstandsankündigung.');
      transfer.bytes=new Uint8Array(length);
      send(room,{t:'archive-next',offset:0});
    }else if(room.owner===0&&message?.t==='archive-next'){
      const {bytes}=room.archive;
      if(transfer.offset>=bytes.length||!exact(message,{t:'archive-next',offset:transfer.offset}))
        throw Error('Ungültige Anfrage für einen Pangea-Spielstandabschnitt.');
      transfer.offset=sendCheckpointWindow(room.ch,bytes,transfer.offset);
      status(`Pangea-Spielstand wird übertragen · ${Math.floor(100*transfer.offset/bytes.length)}%`);
    }else if(room.owner===0&&transfer.offset===room.archive.bytes.length&&
      exact(message,{t:'archive-ready',digest:transfer.digest})){
      ready(room);
    }else throw Error('Ungültige Übertragungsfolge für den Pangea-Spielstand.');
  }

  function receiveArchive(room,packet) {
    const transfer=room.transfer,{bytes,offset}=transfer;
    transfer.offset=receiveCheckpointChunk(bytes,offset,packet);
    if(transfer.offset===bytes.length){
      status('Pangea-Spielstand wird geprüft …');
      void readHuge10Checkpoint(bytes,build,checkSettings).then(archive=>{
        if(active!==room||room.failed||room.transfer!==transfer)return;
        if(archive.digest!==transfer.digest)throw Error('Die Prüfsumme des Pangea-Spielstands stimmt nicht.');
        applyHuge10Checkpoint(wasm,archive,1,true);
        room.archive=archive;
        send(room,{t:'archive-ready',digest:archive.digest});
        ready(room);
      }).catch(error=>fail(room,error.message));
    }else if(transfer.offset%CHECKPOINT_WINDOW===0){
      status(`Pangea-Spielstand wird übertragen · ${Math.floor(100*transfer.offset/bytes.length)}%`);
      send(room,{t:'archive-next',offset:transfer.offset});
    }
  }

  function begin(room) {
    const {ch,pc,owner,settings}=room;
    let linkMs=0,nextMeasure=0;
    const carrier=createSourceCheckpointCarrier(packet=>ch.send(packet),message=>{
      const admitted=room.peer.receive(1-owner,message);
      if (admitted&&message.kind==='abort') fail(room,'Der Mitspieler hat die Pangea-Partie beendet.');
      return admitted;
    });
    room.carrier=carrier;
    const resume=room.archive?.meta??null;
    room.peer=new SourceRoundPeer(wasm,owner,carrier.send,{
      requested:resume?.scale??settings.requested,locked:true,renderFps,linkMs:()=>linkMs,
      nowSeconds:()=>performance.now()/1000,onRequested:()=>{},resume,onCheckpoint:id=>{
        if(wasm.network_naval_epoch_checkpoint_flush()!==1)throw Error('U-Boot-Upgrades konnten nicht gemeinsam übernommen werden.');
        void saveAtBoundary(room,id);
      },
    });
    room.peer.requested=settings.requested;
    const refreshLink=now=>{
      if (owner!==0 || room.failed || now<nextMeasure) return;
      nextMeasure=now+1000;
      pc.getStats().then(stats=>{
        const measurement=selectedRoundTripMilliseconds(stats);
        if (measurement!==null) linkMs=measurement;
      }).catch(()=>{});
    };
    const session={ch,pc,sourceRounds:true,ls:room.peer,wideFresh:true,checkpointHost:owner===0,refreshLink};
    launch(settings,session);
    status('Pangea-Partie gestartet.');
    if (owner===1) room.peer.activate();
  }

  function install(room,settings) {
    if(!room.archive)return prepare(settings,room.owner);
    if(!exact(settings,room.archive.settings))throw Error('Spielstand und Pangea-Einstellungen unterscheiden sich.');
    applyHuge10Checkpoint(wasm,room.archive,room.owner,false);
    return wasm.state_hash()>>>0;
  }

  function finishSave(room,message,archive) {
    clearTimeout(room.save?.timer);
    room.save=null;
    onCheckpoint(message,archive);
  }

  function failSave(room,message) {
    const {id}=room.save;
    send(room,{t:'checkpoint-failed',id});
    finishSave(room,message);
  }

  function settleSave(room) {
    const {id,archive,remote}=room.save;
    if(!archive||remote===undefined)return;
    if(remote!==archive.digest){
      failSave(room,'Die Pangea-Spielstände stimmen nicht überein.');
      return;
    }
    send(room,{t:'checkpoint-ready',id,digest:archive.digest});
    finishSave(room,'Gemeinsamer Pangea-Spielstand bereit.',archive);
  }

  function waitForConfirmation(room,save) {
    save.timer=setTimeout(()=>{
      if(active!==room||room.save!==save)return;
      try{failSave(room,'Die Pangea-Speicherbestätigung ist abgelaufen.');}
      catch(error){fail(room,error.message);}
    },30000);
  }

  async function saveAtBoundary(room,id) {
    if(room.owner===1){
      if(id<=room.lastCheckpoint)return;
      room.lastCheckpoint=id;
      room.save={id};
      onCheckpoint('Gemeinsamer Pangea-Spielstand wird geprüft …');
    }
    const save=room.save;
    if(save?.id!==id)return;
    try{
      save.archive=await captureHuge10Checkpoint(room.peer,room.settings,build);
      if(active!==room||room.save!==save)return;
      if(room.owner===0){
        settleSave(room);
        if(room.save===save)waitForConfirmation(room,save);
      }else send(room,{t:'checkpoint-report',id,digest:save.archive.digest});
    }catch(error){
      if(active!==room||room.save!==save)return;
      try{
        if(room.owner===0)failSave(room,error.message);
        else send(room,{t:'checkpoint-report',id,digest:null});
      }catch(transportError){fail(room,transportError.message);}
    }
  }

  function checkpointControl(room,message) {
    const {id,t,digest}=message??{};
    if(!['checkpoint-report','checkpoint-ready','checkpoint-failed'].includes(t))return false;
    if(!Number.isInteger(id)||id<1||id>0xffffffff)throw Error('Ungültiger Pangea-Speicherbefehl.');
    if(room.owner===0&&t==='checkpoint-report'&&(digest===null||isHuge10Digest(digest))){
      if(room.save?.id!==id)return id<=room.lastCheckpoint;
      if(room.save.remote!==undefined)throw Error('Wiederholte Pangea-Speicherantwort.');
      if(digest===null)failSave(room,'Der Mitspieler konnte den Pangea-Spielstand nicht speichern.');
      else{room.save.remote=digest;settleSave(room);}
      return true;
    }
    if(room.owner===1&&t==='checkpoint-failed'){
      room.lastCheckpoint=Math.max(room.lastCheckpoint,id);
      if(!room.save||room.save.id===id)finishSave(room,'Gemeinsamer Pangea-Spielstand abgelehnt.');
      return true;
    }
    if(room.owner===1&&t==='checkpoint-ready'&&room.save?.id===id&&
      isHuge10Digest(digest)&&room.save.archive?.digest===digest){
      finishSave(room,'Gemeinsamer Pangea-Spielstand bereit.',room.save.archive);
      return true;
    }
    throw Error('Ungültige Pangea-Speicherbestätigung.');
  }

  function requestCheckpoint() {
    const room=active;
    if(!room?.peer||room.failed||room.owner!==0||room.save||room.lastCheckpoint===0xffffffff)return false;
    const id=room.lastCheckpoint+1;
    if(!room.peer.requestCheckpoint(id))return false;
    room.lastCheckpoint=id;
    room.save={id};
    onCheckpoint('Gemeinsamer Pangea-Spielstand wird geprüft …');
    return true;
  }

  function incoming(room,data) {
    if (active!==room || room.failed) return;
    try {
      if (typeof data!=='string') {
        if(room.owner===1&&room.transfer?.bytes)receiveArchive(room,data);
        else if(!room.carrier?.receive(data))throw Error('Ungültiges Pangea-Netzwerkpaket.');
        return;
      }
      if (data.length>4096) throw Error('Ungültige Pangea-Raumdaten.');
      const message=JSON.parse(data);
      if (!room.remote) {
        acceptHello(room,message);
        return;
      }
      if(room.transfer){transferControl(room,message);return;}
      if(!room.ready)throw Error(room.admissionError??'Der Pangea-Raum ist noch nicht bereit.');
      if(room.peer&&checkpointControl(room,message))return;
      if (!room.peer && room.owner===1 && !room.settings && message?.t==='settings') {
        const settings=checkSettings(message.settings);
        if (!exact(message,{t:'settings',settings})) throw Error('Die Pangea-Einstellungen sind ungültig.');
        room.settings=settings;
        room.hash=install(room,settings);
        send(room,{t:'ready',settings,hash:room.hash});
        status('Karte bestätigt. Warte auf den Start.');
      } else if (!room.peer && room.owner===0 && room.settings && message?.t==='ready') {
        if (!hashWord(message.hash) || !exact(message,{t:'ready',settings:room.settings,hash:room.hash})) {
          throw Error('Karte oder Scharmützeleinstellungen unterscheiden sich.');
        }
        send(room,{t:'commit',hash:room.hash});
        begin(room);
      } else if (!room.peer && room.owner===1 && room.settings && exact(message,{t:'commit',hash:room.hash})) {
        begin(room);
      } else {
        throw Error('Ungültige Startfolge im Pangea-Raum.');
      }
    } catch (error) {
      fail(room,error.message);
    }
  }

  function newRoom(owner,id) {
    closeActive();
    const room={owner,id,archive:selectedArchive,lastCheckpoint:0};
    resumeFile.disabled=true;
    room.events={
      open(ch) {
        if (active!==room || room.failed) { ch.close(); return; }
        room.ch=ch;
        if (!ch.ordered || ch.maxPacketLifeTime!==null || ch.maxRetransmits!==null) {
          fail(room,'Pangea braucht eine zuverlässige, geordnete Datenverbindung.');
          return;
        }
        try { send(room,hello(room,owner)); }
        catch (error) { fail(room,error.message); }
      },
      message:data=>incoming(room,data),
      close:()=>fail(room,room.admissionError??'Die Verbindung zum Mitspieler wurde getrennt.'),
    };
    active=room;
    return room;
  }

  host.onclick=async()=>{
    const room=newRoom(0,hex(crypto.getRandomValues(new Uint8Array(16))));
    status('Einladung wird erstellt …');
    try {
      const invitation=await rtcHost(room.events);
      if (active!==room || room.failed) { invitation.pc.close(); return; }
      room.pc=invitation.pc;
      room.accept=invitation.accept;
      output.value=`${prefix.invite}.${room.id}.${invitation.code}`;
      input.placeholder='Antwortcode des Mitspielers';
      connect.textContent='Antwort verbinden';
      connect.disabled=false;
      status('Einladung senden und die Antwort hier einfügen.');
    } catch (error) { fail(room,error.message); }
  };

  guest.onclick=()=>{
    newRoom(1,null);
    input.placeholder='Einladungscode des Gastgebers';
    connect.textContent='Antwort erstellen';
    connect.disabled=false;
    status('Pangea-Einladung einfügen und eine Antwort erstellen.');
  };

  connect.onclick=async()=>{
    const room=active;
    if (!room || room.failed) return;
    try {
      const code=unpackNetworkCode(room.owner===0?'answer':'invite',input.value);
      if (room.owner===0 && code.room!==room.id) throw Error('Die Antwort gehört zu einer anderen Einladung.');
      connect.disabled=true;
      if (room.owner===0) {
        await room.accept(code.sdp);
        if (active===room && !room.failed && !room.remote) status('Verbindung wird geprüft …');
      } else {
        room.id=code.room;
        const answer=await rtcJoin(code.sdp,room.events);
        if (active!==room || room.failed) { answer.pc.close(); return; }
        room.pc=answer.pc;
        output.value=`${prefix.answer}.${room.id}.${answer.code}`;
        if (!room.remote) status('Antwort an den Gastgeber senden.');
      }
    } catch (error) {
      if (active===room && !room.failed) {
        connect.disabled=false;
        status(error.message);
      }
    }
  };

  start.onclick=()=>{
    const room=active;
    if (!room || room.owner!==0 || !room.ready || room.settings || room.failed) return;
    try {
      const settings=room.archive?.settings??checkSettings(makeSettings());
      room.hash=install(room,settings);
      room.settings=settings;
      start.disabled=true;
      send(room,{t:'settings',settings});
      status('Warte auf die Kartenbestätigung des Mitspielers …');
    } catch (error) { fail(room,error.message); }
  };
  get('cancel').onclick=reset;
  resumeFile.onchange=async()=>{
    closeActive();
    const token=++importToken,file=resumeFile.files?.[0];
    selectedArchive=null;
    start.textContent='Partie starten';
    if(!file)return;
    host.disabled=guest.disabled=true;
    status('Pangea-Spielstand wird geprüft …');
    try{
      if(file.size>MAX_HUGE10_SAVE)throw Error('Die Pangea-Spielstandsdatei ist zu groß.');
      const archive=await readHuge10Checkpoint(new Uint8Array(await file.arrayBuffer()),build,checkSettings);
      if(token!==importToken)return;
      applyHuge10Checkpoint(wasm,archive,0,true);
      selectedArchive=archive;
      start.textContent='Spielstand fortsetzen';
      status(`Pangea-Spielstand bei Schritt ${archive.meta.tick} geprüft. Der Gastgeber stellt ihn dem Mitspieler bereit.`);
    }catch(error){
      if(token===importToken){resumeFile.value='';status(error.message);}
    }finally{
      if(token===importToken)host.disabled=guest.disabled=false;
    }
  };
  get('copy').onclick=()=>{
    if (!output.value) return;
    output.select();
    navigator.clipboard?.writeText(output.value).catch(()=>{});
  };
  reset();
  return {reset,requestCheckpoint};
}
