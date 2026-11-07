// A host and independent authenticated RTC links for the optional 3/4-seat mode.
import {createSourceRosterCarrier} from './source-network-wire.js?v=native46';

const PREFIX='OE-R2.';
const VERSION=2;
const integer=(value,min,max)=>Number.isInteger(value)&&value>=min&&value<=max;
const roomToken=/^[0-9a-f-]{36}$/;

function packCode(kind,room,seat,sdp) {
  return PREFIX+btoa(JSON.stringify({kind,room:room.id,seats:room.seats,seat,sdp}));
}

function unpackCode(code,kind) {
  code=code.trim();
  if(!code.startsWith(PREFIX)||code.length>131072)throw Error('Ungültiger Raumcode.');
  const value=JSON.parse(atob(code.slice(PREFIX.length)));
  if(value?.kind!==kind||!roomToken.test(value.room)||!integer(value.seats,3,4)||
    !integer(value.seat,1,value.seats-1)||typeof value.sdp!=='string'||!value.sdp)
    throw Error('Ungültiger Raumcode.');
  return value;
}

export function mountSourceRosterLobby(root,{rtcHost,rtcJoin,localHello,checkHello,makeSettings,checkSettings,
  launch,onOpen,onDeparture,onHostLoss}) {
  const get=id=>root.querySelector('#mp-roster-'+id);
  const status=get('status'),hostRoom=get('host-room'),guestRoom=get('guest-room');
  const startButton=get('start'),countSelect=get('count'),aiSelect=get('ai');
  let current=null;
  const active=(room,link)=>current===room&&room.phase!=='closed'&&(!link||room.links.get(link.seat)===link);
  const connected=room=>[...room.links.values()].filter(link=>link.hello&&link.reachable&&link.channel?.readyState==='open').length+1;
  const setStatus=message=>{status.textContent=message;};

  function reset(message='3 oder 4 Personen mit installiertem UP-Regelprofil.') {
    const old=current;
    current=null;
    if(old){
      old.phase='closed';
      for(const link of old.links.values())link.pc?.close();
    }
    hostRoom.hidden=true;
    guestRoom.hidden=true;
    get('close').hidden=true;
    get('players').replaceChildren();
    get('invite').value='';
    get('answer').value='';
    startButton.disabled=true;
    get('reply').disabled=false;
    setStatus(message);
  }

  function showRoom(role) {
    reset();
    onOpen();
    hostRoom.hidden=role!=='host';
    guestRoom.hidden=role!=='guest';
    get('close').hidden=false;
  }

  function control(link,room,type,details={}) {
    link.channel.send(JSON.stringify({type,version:VERSION,room:room.id,...details}));
  }

  function updateHost(room) {
    const ready=connected(room);
    if(room.phase==='lobby'){
      startButton.disabled=ready!==room.seats;
      setStatus(`${ready}/${room.seats} Plätze bereit.`);
    }
    for(const link of room.links.values())if(link.hello&&link.channel?.readyState==='open')
      control(link,room,'presence',{count:ready});
    room.session?.connected(ready);
  }

  function closeLink(room,link) {
    if(!active(room,link)||link.closed)return;
    link.closed=true;
    link.pc?.close();
    if(room.phase==='preparing'){
      reset('Beim gemeinsamen Spielstart ging eine Verbindung verloren.');
      return;
    }
    if(room.local!==0){
      if(room.phase==='playing')onHostLoss(room.session);
      else reset('Die Verbindung zum Host wurde getrennt.');
      return;
    }
    room.links.delete(link.seat);
    if(room.phase==='playing'){
      room.session.peer.detachPeer(link.seat);
      onDeparture(link.seat);
    }else{
      const row=room.rows.get(link.seat);
      row.state.textContent='Getrennt · neue Einladung möglich';
      row.invite.disabled=false;
      row.connect.disabled=true;
      row.offer.value='';
      row.answer.value='';
    }
    updateHost(room);
  }

  function run(room) {
    const send=message=>{
      for(const link of room.links.values())if(!link.closed)link.carrier.send(message);
    };
    room.session=launch(room.settings,room.local,send,()=>[...room.links.values()]);
    room.phase='playing';
    for(const link of room.links.values()){
      link.carrier=createSourceRosterCarrier(bytes=>{
        if(link.channel.readyState==='open')link.channel.send(bytes);
      },message=>room.session.peer.receive(link.seat,message));
    }
    if(room.local===0){
      room.session.connected(connected(room));
      for(const link of room.links.values())control(link,room,'begin');
    }else room.session.peer.activate();
  }

  function receive(room,link,data) {
    if(!active(room,link)||link.closed)return;
    try{
      if(typeof data!=='string'){
        if(room.phase!=='playing'||!link.carrier?.receive(data))throw Error('Ungültiges Rundensignal.');
        return;
      }
      if(data.length>16384)throw Error('Raumnachricht ist zu groß.');
      const message=JSON.parse(data);
      if(message?.version!==VERSION||message.room!==room.id)throw Error('Die Spielversionen passen nicht zusammen.');
      if(message.type==='hello'&&!link.hello&&room.phase==='lobby'){
        link.hello=checkHello(message.hello);
        if(room.local===0){
          room.rows.get(link.seat).state.textContent=link.reachable?'Bereit':'Verbindung unterbrochen';
          updateHost(room);
        }else setStatus(`Platz ${room.local+1} ist verbunden. Warte auf den Start durch den Host.`);
        return;
      }
      if(!link.hello)throw Error('Der Raum wurde noch nicht bestätigt.');
      if(room.local!==0&&message.type==='presence'&&integer(message.count,1,room.seats)){
        if(room.phase==='playing')room.session.connected(message.count);
        else if(room.phase==='lobby')setStatus(`Platz ${room.local+1} · ${message.count}/${room.seats} Plätze bereit.`);
        return;
      }
      if(room.local!==0&&message.type==='prepare'&&room.phase==='lobby'){
        room.settings=checkSettings(message.settings,room.seats);
        room.phase='preparing';
        setStatus('Einstellungen bestätigt · warte auf den gemeinsamen Start.');
        control(link,room,'prepared');
        return;
      }
      if(room.local===0&&message.type==='prepared'&&room.phase==='preparing'&&!link.prepared){
        link.prepared=true;
        room.rows.get(link.seat).state.textContent='Einstellungen bestätigt';
        if([...room.links.values()].every(entry=>entry.prepared))run(room);
        return;
      }
      if(room.local!==0&&message.type==='begin'&&room.phase==='preparing'){
        run(room);
        return;
      }
      throw Error('Raumnachricht passt nicht zum Spielstand.');
    }catch(error){
      if(room.phase==='playing')room.session.peer.abort(error.message);
      else{
        const failure=error instanceof Error?error.message:'Ungültige Verbindungsdaten.';
        closeLink(room,link);
        setStatus(failure);
      }
    }
  }

  function wire(room,seat) {
    const link={seat,channel:null,pc:null,hello:null,closed:false,reachable:false};
    room.links.set(seat,link);
    const handlers={
      open(channel){
        if(!active(room,link)||link.closed){channel.close();return;}
        link.channel=channel;
        link.reachable=true;
        if(!channel.ordered||channel.maxRetransmits!==null||channel.maxPacketLifeTime!==null){
          closeLink(room,link);
          return;
        }
        control(link,room,'hello',{hello:localHello()});
      },
      message:data=>receive(room,link,data),
      close:()=>closeLink(room,link),
      transport(available){
        if(!active(room,link)||link.closed)return false;
        link.reachable=available;
        if(room.local!==0&&room.phase==='playing')room.session.peer.setTransportAvailable(available);
        if(room.local===0&&link.hello&&room.phase!=='preparing'){
          if(room.phase==='lobby')room.rows.get(seat).state.textContent=available?'Bereit':'Verbindung unterbrochen';
          updateHost(room);
        }else if(room.phase==='lobby'&&room.local!==0&&!available)setStatus('Verbindung zum Host unterbrochen.');
        return true;
      },
    };
    return {link,handlers};
  }

  function addHostRow(room,seat) {
    const field=get('player-template').content.firstElementChild.cloneNode(true);
    field.dataset.seat=seat;
    field.querySelector('legend').textContent=`Spieler ${seat+1}`;
    const part=name=>field.querySelector(`[data-roster-${name}]`);
    const row={invite:part('invite'),connect:part('connect'),state:part('state'),offer:part('offer'),answer:part('answer')};
    room.rows.set(seat,row);
    row.state.textContent='Einladung fehlt';
    row.answer.oninput=()=>{row.connect.disabled=!row.answer.value.trim()||!room.links.get(seat)?.invitation;};
    row.invite.onclick=async()=>{
      row.invite.disabled=true;
      row.state.textContent='Einladung wird erstellt …';
      const {link,handlers}=wire(room,seat);
      try{
        const invitation=await rtcHost(handlers);
        if(!active(room,link)){invitation.pc.close();return;}
        link.invitation=invitation;
        link.pc=invitation.pc;
        row.offer.value=packCode('invite',room,seat,invitation.code);
        row.state.textContent='Warte auf Antwort';
        row.answer.oninput();
      }catch{
        closeLink(room,link);
        if(active(room))row.state.textContent='Einladung fehlgeschlagen';
      }
    };
    row.connect.onclick=async()=>{
      try{
        const answer=unpackCode(row.answer.value,'reply');
        if(answer.room!==room.id||answer.seat!==seat||answer.seats!==room.seats)
          throw Error('Diese Antwort gehört zu einem anderen Platz.');
        row.connect.disabled=true;
        row.state.textContent='Verbindung wird aufgebaut …';
        await room.links.get(seat).invitation.accept(answer.sdp);
      }catch(error){
        row.connect.disabled=false;
        row.state.textContent=error.message;
      }
    };
    get('players').append(field);
  }

  countSelect.onchange=()=>{
    const four=Number(countSelect.value)===4;
    aiSelect.options[1].disabled=four;
    if(four)aiSelect.value='0';
  };
  get('host').onclick=()=>{
    showRoom('host');
    const seats=Number(countSelect.value),ai=Number(aiSelect.value);
    if(!integer(seats,3,4)||!integer(ai,0,4-seats))return;
    const room={id:crypto.randomUUID(),seats,ai,local:0,phase:'lobby',links:new Map(),rows:new Map()};
    current=room;
    for(let seat=1;seat<seats;seat++)addHostRow(room,seat);
    updateHost(room);
  };
  get('join').onclick=()=>{
    showRoom('guest');
    setStatus('Einladungscode für deinen Platz einfügen.');
  };
  get('reply').onclick=async()=>{
    let room;
    try{
      const code=unpackCode(get('invite').value,'invite');
      if(current)throw Error('Schließe zuerst die bisherige Verbindung.');
      room={id:code.room,seats:code.seats,local:code.seat,phase:'lobby',links:new Map()};
      current=room;
      get('reply').disabled=true;
      const {link,handlers}=wire(room,0),answer=await rtcJoin(code.sdp,handlers);
      if(!active(room,link)){answer.pc.close();return;}
      link.pc=answer.pc;
      get('answer').value=packCode('reply',room,room.local,answer.code);
      setStatus(`Antwort für Platz ${room.local+1} an den Host senden.`);
    }catch(error){
      if(room&&active(room)){
        reset(error.message);
        guestRoom.hidden=false;
        get('close').hidden=false;
      }else setStatus(error.message);
    }
  };
  startButton.onclick=()=>{
    const room=current;
    if(!room||room.local!==0||room.phase!=='lobby'||connected(room)!==room.seats)return;
    try{
      const settings=makeSettings(room.seats,room.ai,[...room.links].sort(([left],[right])=>left-right).map(([,link])=>link.hello));
      room.settings=checkSettings(settings,room.seats);
      room.phase='preparing';
      startButton.disabled=true;
      setStatus('Warte auf Bestätigung der Scharmützeleinstellungen.');
      for(const link of room.links.values())control(link,room,'prepare',{settings:room.settings});
    }catch(error){
      if(room.phase==='preparing')reset(error.message);
      else setStatus(error.message);
    }
  };
  get('close').onclick=()=>reset();
  root.hidden=false;
  return {reset};
}
