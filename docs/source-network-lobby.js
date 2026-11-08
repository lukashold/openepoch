// A host and independent authenticated RTC links for roster and shared-owner rooms.
import {createSourceRosterCarrier,createSourceCheckpointCarrier,createSourceSharedOwnerCarrier,
  createSourceGroupedCoopCarrier} from './source-network-wire.js?v=native54';
import {captureBrowserCheckpoint,readBrowserCheckpoint,sendCheckpointWindow,receiveCheckpointChunk,
  validCheckpointDigest,validCheckpointSize,CHECKPOINT_WINDOW} from './source-network-checkpoint.js?v=native54';
import {isLanInvite,lanStatus,publishLanInvite,claimLanInvite} from './lan-signaling.js?v=native56';
import {mountLanRoomList} from './lan-room-list.js?v=native56';

const PREFIX='OE-R2.';
const VERSION=2;
const integer=(value,min,max)=>Number.isInteger(value)&&value>=min&&value<=max;
const roomToken=/^[0-9a-f-]{36}$/;

export function requiresCapitolRoster(mode,humanSeats,ai) {
  return mode!==5||humanSeats!==2||ai!==0;
}

function packCode(kind,room,seat,sdp) {
  return PREFIX+btoa(JSON.stringify({kind,room:room.id,seats:room.seats,seat,sdp}));
}

function unpackCode(code,kind) {
  code=code.trim();
  if(!code.startsWith(PREFIX)||code.length>131072)throw Error('Ungültiger Raumcode.');
  const value=JSON.parse(atob(code.slice(PREFIX.length)));
  if(value?.kind!==kind||!roomToken.test(value.room)||!integer(value.seats,2,4)||
    !integer(value.seat,1,value.seats-1)||typeof value.sdp!=='string'||!value.sdp)
    throw Error('Ungültiger Raumcode.');
  return value;
}

export function mountSourceRosterLobby(root,{rtcHost,rtcJoin,localHello,checkHello,makeSettings,checkSettings,onHostRoster,
  launch,onOpen,onDeparture,onHostLoss,validateArchive,onCheckpointState,sharedOwnerAvailable,sharedCheckpointAvailable,
  groupedCoopAvailable,groupedCheckpointAvailable,describeRoom,epochName}) {
  const get=id=>root.querySelector('#mp-roster-'+id);
  const status=get('status'),hostRoom=get('host-room'),guestRoom=get('guest-room');
  const startButton=get('start'),countSelect=get('count'),aiSelect=get('ai');
  let current=null,guestRequest=null,lanHost=null,lanAvailable=false;
  const active=(room,link)=>current===room&&room.phase!=='closed'&&(!link||room.links.get(link.seat)===link);
  const connected=room=>[...room.links.values()].filter(link=>link.hello&&link.reachable&&link.channel?.readyState==='open').length+1;
  const setStatus=message=>{status.textContent=message;};
  const directory=mountLanRoomList(get,{
    epochName,
    details(name){
      const room=current;
      if(room?.local!==0||room.phase!=='lobby')return null;
      return {room:room.id,name,seats:room.seats,ready:connected(room),open:room.seats-1-room.links.size,ai:room.ai,
        mode:room.groupedOwners?'grouped':room.sharedOwner?'shared':'ordinary',tick:room.archive?.decoded.meta.tick??null,
        ...describeRoom(room.archive?.settings)};
    },
    approve(owner,request){
      const room=current;
      if(room?.local!==0||room.phase!=='lobby')throw Error('Der Raum nimmt keine neuen Spieler mehr auf.');
      const available=[...room.rows].find(([seat])=>!room.links.has(seat));
      if(!available)throw Error('In diesem LAN-Raum ist gerade kein Platz frei.');
      return available[1].beginInvitation(true,{owner,request});
    },
    join(code){showRoom('guest');void joinInvitation(code);},
  });
  window.addEventListener('pagehide',()=>directory.reset());
  void lanStatus().then(lan=>{
    lanAvailable=lan.available===true;
    lanHost=typeof lan.host==='string'?lan.host:null;
    get('lan-help').hidden=!lanAvailable;
    get('lan-help').textContent=lanHost?`Direktes LAN aktiv · ${lanHost}`:
      'LAN-Beitritt verfügbar. Zum Hosten den Spielstarter mit --lan und deiner privaten IPv4-Adresse starten.';
    for(const row of current?.rows?.values()??[])row.lan.hidden=!lanHost;
    directory.availability(lan);
    if(current?.local===0&&current.phase==='lobby')directory.showHost();
  });

  function reset(message='2 bis 4 Personen mit installiertem UP-Regelprofil.') {
    const old=current;
    current=null;
    guestRequest=null;
    directory.reset();
    if(old){
      old.phase='closed';
      clearTimeout(old.prepareTimer);
      clearTimeout(old.save?.timer);
      for(const link of old.links.values()){
        clearTimeout(link.lanTimer);
        link.lanTicket?.close();
        link.pc?.close();
      }
    }
    hostRoom.hidden=true;
    guestRoom.hidden=true;
    get('close').hidden=true;
    get('players').replaceChildren();
    get('invite').value='';
    get('answer').value='';
    get('manual-answer').hidden=false;
    get('resume-file').value='';
    startButton.textContent='Partie starten';
    startButton.disabled=true;
    get('reply').disabled=false;
    get('reply').textContent='Antwort erzeugen';
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

  function archiveAdmissionError(room,guests) {
    if(!room.archive)return '';
    if(room.sharedOwner&&(!sharedCheckpointAvailable||guests.some(guest=>!guest.sharedOwnerCheckpoint)))
      return 'Der Mitspieler benötigt eine Spielversion mit Koop-Spielständen.';
    if(room.groupedOwners&&(!groupedCheckpointAvailable||guests.some(guest=>!guest.groupedCheckpoint)))
      return 'Alle Mitspieler benötigen eine Spielversion mit Gruppen-Koop-Spielständen.';
    if(!room.sharedOwner&&!room.groupedOwners&&(!validateArchive||guests.some(guest=>!guest.checkpoints)))
      return 'Alle Mitspieler benötigen eine Spielversion mit Mehrspieler-Spielständen.';
    const {climate,features,crownVictory,capitolVictory,humanSeats,ai,captureCheckpoint,startingTeams,sightTerms}=room.archive.settings;
    if(crownVictory&&guests.some(guest=>!guest.crownVictory))
      return 'Alle Mitspieler benötigen eine Spielversion mit Kronen-Siegbedingungen.';
    if(capitolVictory&&guests.some(guest=>!guest.capitolVictory||
      (requiresCapitolRoster(capitolVictory.mode,humanSeats,ai)&&!guest.capitolRoster)))
      return 'Alle Mitspieler benötigen eine Spielversion mit diesen Kapitol-Siegbedingungen.';
    if(captureCheckpoint&&guests.some(guest=>!guest.captureCheckpoint))
      return 'Alle Mitspieler benötigen eine Spielversion mit Eroberungen nach dem Laden.';
    if(startingTeams&&guests.some(guest=>!guest.startingTeams))
      return 'Alle Mitspieler benötigen eine Spielversion mit Startteams.';
    if(sightTerms&&guests.some(guest=>!guest.sightTerms))
      return 'Alle Mitspieler benötigen eine Spielversion mit diesen Sichtregeln.';
    if(guests.some(guest=>!guest.climates.includes(climate)||features.some(key=>!guest.features.includes(key))))
      return 'Einem Mitspieler fehlen Regeln oder Grafiken für diesen Spielstand.';
    return '';
  }

  function updateHost(room) {
    const ready=connected(room);
    if(room.phase==='lobby'){
      const guests=[...room.links.values()].map(link=>link.hello).filter(Boolean);
      const incompatible=room.sharedOwner&&guests.some(guest=>!guest.sharedOwner);
      const missingGroups=room.groupedOwners&&guests.some(guest=>!guest.groupedCoop);
      const archiveError=archiveAdmissionError(room,guests);
      startButton.disabled=ready!==room.seats||incompatible||missingGroups||!!archiveError;
      const count=`${ready}/${room.seats} Plätze`;
      if(incompatible)setStatus('Der Mitspieler benötigt eine Spielversion mit gemeinsamer Zivilisation.');
      else if(missingGroups)setStatus('Alle Mitspieler benötigen eine Spielversion mit Gruppen-Koop.');
      else if(archiveError)setStatus(archiveError);
      else if(room.groupedOwners&&room.archive)setStatus(`Gruppen-Koop-Spielstand bei Schritt ${room.archive.decoded.meta.tick} · ${ready}/${room.seats} Controller bereit.`);
      else if(room.groupedOwners)setStatus(`Gruppen-Koop · ${ready}/${room.seats} Controller bereit.`);
      else if(room.sharedOwner&&room.archive)setStatus(`Koop-Spielstand bei Schritt ${room.archive.decoded.meta.tick} · ${ready}/${room.seats} Controller bereit.`);
      else if(room.sharedOwner)setStatus(`Gemeinsame Zivilisation gegen die KI · ${count} bereit.`);
      else setStatus(room.archive?`Spielstand bei Schritt ${room.archive.decoded.meta.tick} · ${count} verbunden.`:`${count} bereit.`);
    }
    for(const link of room.links.values())if(link.hello&&link.channel?.readyState==='open')
      control(link,room,'presence',{count:ready});
    room.session?.connected(ready);
  }

  function closeLink(room,link) {
    if(!active(room,link)||link.closed)return;
    link.closed=true;
    clearTimeout(link.lanTimer);
    link.lanTicket?.close();
    link.pc?.close();
    if(room.phase==='preparing'){
      reset('Beim gemeinsamen Spielstart ging eine Verbindung verloren.');
      return;
    }
    if(room.local!==0){
      if(room.phase==='playing'){
        if(room.settings.sharedOwner||room.settings.groupedOwners)room.session.connected(1);
        onHostLoss(room.session);
      }
      else reset('Die Verbindung zum Host wurde getrennt.');
      return;
    }
    room.links.delete(link.seat);
    if(room.phase==='playing'){
      if(room.save)failCheckpoint(room,'Spielstand konnte nicht von allen bestätigt werden.');
      if(room.settings.sharedOwner)room.session.peer.detachSharedController(link.seat);
      else if(room.seats>2||room.settings.groupedOwners)room.session.peer.detachPeer(link.seat);
      else{
        room.session.peer.rosterUnavailable=true;
        room.session.peer.setTransportAvailable(false);
      }
      onDeparture(link.seat,!!room.settings.groupedOwners);
    }else{
      const row=room.rows.get(link.seat);
      row.state.textContent='Getrennt · neue Einladung möglich';
      row.invite.disabled=false;
      row.lan.disabled=false;
      row.connect.disabled=true;
      row.offer.value='';
      row.answer.value='';
      row.manual.hidden=false;
    }
    updateHost(room);
  }

  function run(room) {
    if(room.local===0&&connected(room)!==room.seats)throw Error('Beim Spielstart ging eine Verbindung verloren.');
    clearTimeout(room.prepareTimer);
    const send=message=>{
      for(const link of room.links.values())if(!link.closed)link.carrier.send(message);
    };
    const onCheckpoint=room.checkpoints?id=>saveAtBoundary(room,id):null;
    room.session=launch(room.settings,room.local,send,()=>[...room.links.values()],{archive:room.archive,onCheckpoint});
    room.phase='playing';
    room.archive=null;
    const makeCarrier=room.checkpoints?createSourceCheckpointCarrier:createSourceRosterCarrier;
    for(const link of room.links.values()){
      const sendBytes=bytes=>{
        if(link.channel.readyState==='open')link.channel.send(bytes);
      };
      const receiveMessage=message=>room.session.peer.receive(link.seat,message);
      link.carrier=room.settings.sharedOwner?createSourceSharedOwnerCarrier(sendBytes,receiveMessage,room.checkpoints)
        :room.settings.groupedOwners?createSourceGroupedCoopCarrier(sendBytes,receiveMessage,room.checkpoints):makeCarrier(sendBytes,receiveMessage);
    }
    if(room.local===0){
      room.session.connected(connected(room));
      for(const link of room.links.values())control(link,room,'begin');
    }else room.session.peer.activate();
  }

  function armPreparation(room) {
    room.prepareTimer=setTimeout(()=>{
      if(active(room)&&room.phase==='preparing')reset('Der gemeinsame Spielstart hat zu lange gedauert.');
    },120000);
  }

  function confirmPrepared(room,link,digest=null) {
    room.prepared=true;
    setStatus(digest?'Spielstand geprüft · warte auf den gemeinsamen Start.':'Einstellungen bestätigt · warte auf den gemeinsamen Start.');
    control(link,room,'prepared',{digest});
  }

  function receiveArchive(room,link,packet) {
    const download=link.download;
    download.offset=receiveCheckpointChunk(download.bytes,download.offset,packet);
    const percent=Math.floor(100*download.offset/download.bytes.length);
    setStatus(`Spielstand wird übertragen · ${percent}%`);
    if(download.offset===download.bytes.length){
      link.download=null;
      void readBrowserCheckpoint(download.bytes,download.digest).then(archive=>{
        if(!active(room,link)||room.phase!=='preparing')return;
        if((archive.decoded.meta.sharedOwner===true)!==room.expectedSharedOwner ||
            !!archive.decoded.meta.groupedOwners!==room.expectedGroupedOwners)
          throw Error('Der Spielstand hat eine andere Spielerzuordnung.');
        room.settings=checkSettings(archive.settings,room.seats,true);
        if(room.settings.startingTeams&&!link.hello.startingTeams)throw Error('Der Host hat Startteams nicht angekündigt.');
        if(room.settings.sightTerms&&!link.hello.sightTerms)throw Error('Der Host hat diese Sichtregeln nicht angekündigt.');
        validateArchive(archive,room.local,room.seats);
        room.archive=archive;
        confirmPrepared(room,link,archive.digest);
      }).catch(error=>{
        if(active(room,link)){closeLink(room,link);setStatus(error.message);}
      });
    }else if(download.offset%CHECKPOINT_WINDOW===0)control(link,room,'archive-next',{offset:download.offset});
  }

  function failCheckpoint(room,message) {
    const save=room.save;
    if(!save)return;
    clearTimeout(save.timer);
    room.save=null;
    if(room.local===0)for(const link of room.links.values())if(link.channel?.readyState==='open')
      control(link,room,'checkpoint-failed',{id:save.id});
    onCheckpointState(message);
  }

  function settleCheckpoint(room) {
    const save=room.save;
    if(!save?.archive)return;
    if([...save.reports.values()].some(digest=>digest!==save.archive.digest)){
      failCheckpoint(room,'Die Spielstände der Mitspieler stimmen nicht überein.');
      return;
    }
    const checked=save.reports.size+1;
    if(checked!==save.expected){onCheckpointState(`Spielstand geprüft · ${checked}/${save.expected} Plätze.`);return;}
    clearTimeout(save.timer);
    room.save=null;
    for(const link of room.links.values())control(link,room,'checkpoint-ready',{id:save.id,digest:save.archive.digest});
    onCheckpointState('Gemeinsamer Spielstand bereit.',save.archive);
  }

  function saveAtBoundary(room,id) {
    if(room.local!==0){
      if(id<=(room.lastCheckpoint??0))throw Error('Wiederholter Speicherbefehl.');
      room.lastCheckpoint=id;
      room.save={id};
      onCheckpointState('Gemeinsamer Spielstand wird geprüft …');
    }
    const save=room.save;
    if(save?.id!==id)return;
    void captureBrowserCheckpoint(room.session.peer,room.settings).then(archive=>{
      if(!active(room)||room.save!==save)return;
      save.archive=archive;
      if(room.local===0)settleCheckpoint(room);
      else control(room.links.get(0),room,'checkpoint-report',{id,digest:archive.digest});
    }).catch(()=>{
      if(!active(room)||room.save!==save)return;
      if(room.local===0)failCheckpoint(room,'Dieser Spielstand kann gerade nicht gespeichert werden.');
      else control(room.links.get(0),room,'checkpoint-report',{id,digest:null});
    });
  }

  function receiveCheckpointStatus(room,link,message) {
    if(!room.checkpoints)return false;
    if(room.local===0&&message.type==='checkpoint-report'){
      if(room.save?.id===message.id){
        if(message.digest===null){failCheckpoint(room,'Ein Mitspieler konnte den Spielstand nicht speichern.');return true;}
        if((room.sharedOwner||room.groupedOwners)&&room.save.reports.has(link.seat))return room.save.reports.get(link.seat)===message.digest;
        if(!validCheckpointDigest(message.digest)||room.save.reports.has(link.seat))return false;
        room.save.reports.set(link.seat,message.digest);
        settleCheckpoint(room);
      }
      return true;
    }
    if(room.local!==0&&message.type==='checkpoint-failed'){
      if(room.save?.id===message.id)failCheckpoint(room,'Der gemeinsame Spielstand konnte nicht bestätigt werden.');
      return true;
    }
    if(room.local!==0&&message.type==='checkpoint-ready'){
      const save=room.save;
      if(save?.id!==message.id||save.archive?.digest!==message.digest)return false;
      room.save=null;
      onCheckpointState('Gemeinsamer Spielstand bereit.',save.archive);
      return true;
    }
    return false;
  }

  function receive(room,link,data) {
    if(!active(room,link)||link.closed)return;
    try{
      if(typeof data!=='string'){
        if(room.phase==='preparing'&&room.local!==0&&link.download){receiveArchive(room,link,data);return;}
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
        if(room.settings.startingTeams&&!link.hello.startingTeams)throw Error('Der Host hat Startteams nicht angekündigt.');
        if(room.settings.sightTerms&&!link.hello.sightTerms)throw Error('Der Host hat diese Sichtregeln nicht angekündigt.');
        room.checkpoints=message.checkpoints===true&&!!validateArchive;
        if(message.checkpoints===true&&!room.checkpoints)throw Error('Diese Spielversion kann gemeinsame Spielstände nicht öffnen.');
        if(room.settings.sharedOwner&&(!link.hello.sharedOwner||(room.checkpoints&&
            (!sharedCheckpointAvailable||!link.hello.sharedOwnerCheckpoint))))
          throw Error('Ungültige Einstellungen für die gemeinsame Zivilisation.');
        if(room.settings.groupedOwners&&(!link.hello.groupedCoop||(room.checkpoints&&
            (!groupedCheckpointAvailable||!link.hello.groupedCheckpoint))))
          throw Error('Ungültige Einstellungen für Gruppen-Koop.');
        room.phase='preparing';
        armPreparation(room);
        confirmPrepared(room,link);
        return;
      }
      if(room.local!==0&&message.type==='prepare-resume'&&room.phase==='lobby'&&validateArchive){
        if(!validCheckpointSize(message.length)||!validCheckpointDigest(message.digest))throw Error('Ungültige Spielstandsankündigung.');
        room.expectedSharedOwner=message.sharedOwner===true;
        room.expectedGroupedOwners=message.groupedCoop===true;
        if(message.sharedOwner!==undefined&&(!room.expectedSharedOwner||!sharedCheckpointAvailable||!link.hello.sharedOwnerCheckpoint))
          throw Error('Diese Spielversion kann Koop-Spielstände nicht öffnen.');
        if(message.groupedCoop!==undefined&&(!room.expectedGroupedOwners||room.expectedSharedOwner||
            !groupedCheckpointAvailable||!link.hello.groupedCheckpoint))
          throw Error('Diese Spielversion kann Gruppen-Koop-Spielstände nicht öffnen.');
        room.checkpoints=true;
        room.phase='preparing';
        armPreparation(room);
        link.download={bytes:new Uint8Array(message.length),offset:0,digest:message.digest};
        setStatus('Warte auf den Spielstand des Hosts …');
        control(link,room,'archive-next',{offset:0});
        return;
      }
      if(room.local===0&&message.type==='archive-next'&&room.phase==='preparing'&&room.archive){
        const offset=link.archiveOffset??0;
        if(message.offset!==offset||offset>=room.archive.bytes.length)throw Error('Ungültige Anfrage für einen Spielstandabschnitt.');
        link.archiveOffset=sendCheckpointWindow(link.channel,room.archive.bytes,offset);
        const percent=Math.floor(100*link.archiveOffset/room.archive.bytes.length);
        room.rows.get(link.seat).state.textContent=`Spielstand übertragen · ${percent}%`;
        return;
      }
      if(room.local===0&&message.type==='prepared'&&room.phase==='preparing'&&!link.prepared){
        if(room.archive&&(message.digest!==room.archive.digest||link.archiveOffset!==room.archive.bytes.length))
          throw Error('Der Mitspieler hat den Spielstand nicht bestätigt.');
        link.prepared=true;
        link.preparedDigest=message.digest;
        room.rows.get(link.seat).state.textContent=room.archive?'Spielstand geprüft':'Einstellungen bestätigt';
        if([...room.links.values()].every(entry=>entry.prepared))run(room);
        return;
      }
      if(room.local===0&&(room.sharedOwner||room.groupedOwners)&&message.type==='prepared'&&link.prepared&&message.digest===link.preparedDigest)return;
      if(room.local!==0&&message.type==='begin'&&room.phase==='preparing'&&room.prepared){
        run(room);
        return;
      }
      if(room.phase==='playing'&&receiveCheckpointStatus(room,link,message))return;
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
        clearTimeout(link.lanTimer);
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
    field.querySelector('legend').textContent=room.groupedOwners
      ?`Controller ${seat+1} · Zivilisation ${room.groupedOwners[seat]+1}`:`Spieler ${seat+1}`;
    const part=name=>field.querySelector(`[data-roster-${name}]`);
    const row={invite:part('invite'),lan:part('lan'),connect:part('connect'),state:part('state'),offer:part('offer'),
      answer:part('answer'),manual:part('manual')};
    room.rows.set(seat,row);
    row.lan.hidden=!lanHost;
    row.state.textContent='Einladung fehlt';
    row.answer.oninput=()=>{row.connect.disabled=!row.answer.value.trim()||!room.links.get(seat)?.invitation;};
    async function beginInvitation(local,admission=null){
      row.invite.disabled=true;
      row.lan.disabled=true;
      row.state.textContent='Einladung wird erstellt …';
      const {link,handlers}=wire(room,seat);
      try{
        const invitation=await rtcHost(handlers,local);
        if(!active(room,link)){invitation.pc.close();return;}
        link.pc=invitation.pc;
        if(local){
          const ticket=await publishLanInvite(lanHost,{room:room.id,seat,seats:room.seats},invitation.code);
          if(!active(room,link)){ticket.close();return;}
          link.lanTicket=ticket;
          row.manual.hidden=true;
          if(admission){
            await ticket.approve(admission.owner,admission.request);
            if(!active(room,link))return;
            row.state.textContent='LAN · Freigegeben · warte auf Verbindung';
          }else{
            row.offer.value=ticket.code;
            row.state.textContent='LAN · Warte auf Beitritt';
          }
          const answer=await ticket.answer();
          if(!answer||!active(room,link))return;
          row.state.textContent='LAN · Verbindung wird aufgebaut …';
          link.lanTimer=setTimeout(()=>{
            if(!active(room,link)||link.channel?.readyState==='open')return;
            closeLink(room,link);
            if(active(room))row.state.textContent='LAN-Verbindung konnte nicht aufgebaut werden.';
          },30000);
          await invitation.accept(answer);
          ticket.close();
        }else{
          link.invitation=invitation;
          row.offer.value=packCode('invite',room,seat,invitation.code);
          row.state.textContent='Warte auf Antwort';
          row.answer.oninput();
        }
      }catch(error){
        if(active(room,link)){
          closeLink(room,link);
          if(active(room))row.state.textContent=local?error.message:'Einladung fehlgeschlagen';
        }
        if(admission)throw error;
      }
    }
    row.beginInvitation=beginInvitation;
    row.invite.onclick=()=>beginInvitation(false);
    row.lan.onclick=()=>beginInvitation(true);
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
    get('grouped').disabled=Number(countSelect.value)===2;
    get('group-options').hidden=!groupedCoopAvailable||!four;
  };
  countSelect.onchange();
  function finishHostRoom(room,seats,ai,archive=null,sharedOwner=false,groupedOwners=null) {
    Object.assign(room,{seats,ai,archive,sharedOwner,groupedOwners,phase:'lobby'});
    onHostRoster?.(seats,ai,!archive&&!sharedOwner&&!groupedOwners);
    startButton.textContent=archive?'Spielstand fortsetzen':sharedOwner?'Koop-Partie starten':
      groupedOwners?'Gruppen-Koop starten':'Partie starten';
    for(let seat=1;seat<room.seats;seat++)addHostRow(room,seat);
    directory.showHost();
    updateHost(room);
  }

  function openHostRoom() {
    showRoom('host');
    const room={id:crypto.randomUUID(),local:0,phase:'reading',links:new Map(),rows:new Map(),lastCheckpoint:0};
    current=room;
    return room;
  }

  get('host').onclick=()=>{
    const seats=Number(countSelect.value),ai=Number(aiSelect.value);
    if(!integer(seats,validateArchive?2:3,4)||!integer(ai,0,Math.min(1,4-seats)))return;
    finishHostRoom(openHostRoom(),seats,ai);
  };
  get('coop').hidden=!sharedOwnerAvailable;
  get('coop').onclick=()=>{
    if(!sharedOwnerAvailable)return;
    countSelect.value='2';
    countSelect.onchange();
    aiSelect.value='1';
    finishHostRoom(openHostRoom(),2,1,null,true);
  };
  get('grouped').hidden=!groupedCoopAvailable;
  get('grouped').onclick=()=>{
    const seats=Number(countSelect.value),ai=Number(aiSelect.value);
    if(!groupedCoopAvailable||!integer(seats,3,4)||!integer(ai,0,1))return;
    const layouts={pairs:[0,0,1,1],singlePair:[0,0,1,2],trio:[0,0,0,1]};
    const owners=seats===3?[0,0,1]:layouts[get('group-layout').value];
    if(owners)finishHostRoom(openHostRoom(),seats,ai,null,false,owners);
  };
  get('resume-file').onchange=async()=>{
    const file=get('resume-file').files[0];
    if(!validateArchive||!file)return;
    const room=openHostRoom();
    setStatus('Spielstand wird geprüft …');
    try{
      if(!validCheckpointSize(file.size))throw Error('Ungültige Spielstandsgröße.');
      const archive=await readBrowserCheckpoint(new Uint8Array(await file.arrayBuffer()));
      if(!active(room))return;
      const {humanSeats}=archive.decoded.meta;
      const settings=checkSettings(archive.settings,humanSeats,true);
      validateArchive(archive,0,humanSeats);
      finishHostRoom(room,humanSeats,settings.ai,archive,settings.sharedOwner===true,settings.groupedOwners??null);
    }catch(error){if(active(room))reset(error.message);}
  };
  get('join').onclick=()=>{
    showRoom('guest');
    setStatus(lanAvailable?'LAN- oder manuellen Einladungscode für deinen Platz einfügen.':'Einladungscode für deinen Platz einfügen.');
  };
  get('browse').onclick=()=>{
    showRoom('browse');
    void directory.startBrowsing();
  };
  get('invite').oninput=()=>{
    get('reply').textContent=isLanInvite(get('invite').value)?'Über LAN beitreten':'Antwort erzeugen';
  };
  async function joinInvitation(invitation){
    let room,claim,attempt;
    try{
      if(current)throw Error('Schließe zuerst die bisherige Verbindung.');
      attempt={};
      guestRequest=attempt;
      const local=isLanInvite(invitation);
      get('reply').disabled=true;
      if(local){
        get('manual-answer').hidden=true;
        get('reply').textContent='Über LAN beitreten';
        setStatus('LAN-Host wird kontaktiert …');
        claim=await claimLanInvite(invitation);
        if(guestRequest!==attempt){claim.release();return;}
      }
      const code=claim??unpackCode(invitation,'invite');
      if(code.seats===2&&!validateArchive)throw Error('Dieser Raum benötigt eine neuere Spielversion.');
      room={id:code.room,seats:code.seats,local:code.seat,phase:'lobby',links:new Map()};
      current=room;
      const {link,handlers}=wire(room,0),answer=await rtcJoin(code.sdp,handlers,local);
      if(!active(room,link)){answer.pc.close();claim?.release();return;}
      link.pc=answer.pc;
      if(local){
        setStatus(`LAN · Platz ${room.local+1} wird verbunden …`);
        link.lanTimer=setTimeout(()=>{
          if(!active(room,link)||link.channel?.readyState==='open')return;
          reset('LAN-Verbindung konnte nicht aufgebaut werden. Lass dir eine neue Einladung geben.');
          guestRoom.hidden=false;
          get('close').hidden=false;
        },30000);
        await claim.answer(answer.code);
      }else{
        get('answer').value=packCode('reply',room,room.local,answer.code);
        setStatus(`Antwort für Platz ${room.local+1} an den Host senden.`);
      }
    }catch(error){
      claim?.release();
      if(attempt&&guestRequest!==attempt)return;
      if(room&&active(room)){
        reset(error.message);
        guestRoom.hidden=false;
        get('close').hidden=false;
      }else{
        get('reply').disabled=false;
        setStatus(error.message);
      }
    }
  }
  get('reply').onclick=()=>void joinInvitation(get('invite').value);
  startButton.onclick=()=>{
    const room=current;
    if(!room||room.local!==0||room.phase!=='lobby'||connected(room)!==room.seats)return;
    try{
      const guests=[...room.links].sort(([left],[right])=>left-right).map(([,link])=>link.hello);
      if(room.sharedOwner&&!guests.every(guest=>guest.sharedOwner))throw Error('Der Mitspieler benötigt eine Spielversion mit gemeinsamer Zivilisation.');
      if(room.groupedOwners&&!guests.every(guest=>guest.groupedCoop))throw Error('Alle Mitspieler benötigen eine Spielversion mit Gruppen-Koop.');
      const archiveError=archiveAdmissionError(room,guests);
      if(archiveError)throw Error(archiveError);
      room.checkpoints=room.sharedOwner?sharedCheckpointAvailable&&guests.every(guest=>guest.sharedOwnerCheckpoint)
        :room.groupedOwners?groupedCheckpointAvailable&&guests.every(guest=>guest.groupedCheckpoint)
          :!!validateArchive&&guests.every(guest=>guest.checkpoints);
      const settings=room.archive?.settings??makeSettings(room.seats,room.ai,guests,room.sharedOwner,room.groupedOwners);
      room.settings=checkSettings(settings,room.seats,!!room.archive);
      directory.stopHosting();
      room.phase='preparing';
      startButton.disabled=true;
      armPreparation(room);
      if(room.archive){
        setStatus('Warte, bis alle Mitspieler den Spielstand geprüft haben.');
        for(const link of room.links.values())control(link,room,'prepare-resume',{length:room.archive.bytes.length,digest:room.archive.digest,
          ...(room.sharedOwner?{sharedOwner:true}:room.groupedOwners?{groupedCoop:true}:{})});
      }else{
        setStatus('Warte auf Bestätigung der Scharmützeleinstellungen.');
        for(const link of room.links.values())control(link,room,'prepare',{settings:room.settings,checkpoints:room.checkpoints});
      }
    }catch(error){
      if(room.phase==='preparing')reset(error.message);
      else setStatus(error.message);
    }
  };
  get('close').onclick=()=>reset();
  root.hidden=false;
  return {reset,requestCheckpoint(){
    const room=current;
    if(!room?.checkpoints||room.phase!=='playing'||room.local!==0||room.save)return false;
    const expected=room.groupedOwners?room.links.size+1:room.seats;
    if(expected<2||connected(room)!==expected)return false;
    const id=room.lastCheckpoint+1;
    if(!room.session.peer.requestCheckpoint(id))return false;
    room.lastCheckpoint=id;
    const timer=setTimeout(()=>{if(active(room)&&room.save?.id===id)failCheckpoint(room,'Spielstand konnte nicht rechtzeitig bestätigt werden.');},45000);
    room.save={id,timer,expected,reports:new Map()};
    onCheckpointState('Gemeinsamer Spielstand wird erstellt …');
    return true;
  }};
}
