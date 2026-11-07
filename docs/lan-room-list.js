import {lanInviteCode,lanRequest} from './lan-signaling.js?v=native56';

const wait=()=>new Promise(done=>setTimeout(done,750));
const ignore=promise=>void promise.catch(()=>{});
const modeNames={ordinary:'Scharmützel',shared:'Gemeinsame Zivilisation',grouped:'Gruppen-Koop'};
const climateNames=['Gemäßigt','Trocken','Tropisch','Tundra'];

export function mountLanRoomList(get,{details,approve,join,epochName}) {
  const hostPanel=get('lan-host'),browsePanel=get('lan-browser'),hostStatus=get('lan-host-status');
  const pendingList=get('lan-pending'),roomsList=get('lan-rooms'),browseStatus=get('lan-browse-status');
  const toggle=get('lan-list'),cancel=get('lan-cancel-request');
  let publishing=null,browsing=null,joining=null,canHost=false;

  function button(label,action){
    const node=document.createElement('button');
    node.type='button';node.className='alt';node.textContent=label;node.onclick=action;
    return node;
  }

  function stopHosting(message='Dieser Raum ist nur über Einladungen erreichbar.'){
    const session=publishing;
    publishing=null;
    if(session?.owner)ignore(lanRequest('room-close',{owner:session.owner},true));
    pendingList.replaceChildren();
    toggle.disabled=false;toggle.textContent='Im LAN anzeigen';
    get('lan-room-name').disabled=false;
    hostStatus.textContent=message;
  }

  function updatePending(session,pending,open){
    const key=JSON.stringify([pending,open,[...session.deciding]]);
    if(session.pendingKey===key)return;
    session.pendingKey=key;
    pendingList.replaceChildren();
    for(const entry of pending){
      const row=document.createElement('div');row.className='row';
      const name=document.createElement('span');name.textContent=`${entry.name} · ${entry.address}`;
      const accept=button('Platz freigeben',()=>void decide(true));
      const reject=button('Ablehnen',()=>void decide(false));
      const busy=session.deciding.has(entry.request);
      accept.disabled=busy||open===0;reject.disabled=busy;
      row.append(name,accept,reject);pendingList.append(row);
      async function decide(admit){
        if(publishing!==session||session.deciding.has(entry.request))return;
        session.deciding.add(entry.request);
        accept.disabled=reject.disabled=true;
        try{
          if(admit)await approve(session.owner,entry.request);
          else await lanRequest('room-reject',{owner:session.owner,request:entry.request});
        }catch(error){if(publishing===session)hostStatus.textContent=error.message;}
        finally{session.deciding.delete(entry.request);session.pendingKey=null;}
      }
    }
  }

  toggle.onclick=async()=>{
    if(publishing){stopHosting();return;}
    const name=get('lan-room-name').value.trim();
    const initial=details(name);
    if(!canHost||!initial)return;
    const session={owner:null,deciding:new Set(),pendingKey:null};
    publishing=session;
    toggle.disabled=true;get('lan-room-name').disabled=true;
    try{
      const {owner}=await lanRequest('room-open',{details:initial});
      if(publishing!==session){ignore(lanRequest('room-close',{owner},true));return;}
      session.owner=owner;
      toggle.disabled=false;toggle.textContent='LAN-Anzeige beenden';
      hostStatus.textContent='Im LAN sichtbar. Neue Mitspieler benötigen deine Freigabe.';
      while(publishing===session){
        const next=details(name);
        if(!next){stopHosting();return;}
        const {pending}=await lanRequest('room-update',{owner,details:next});
        if(publishing!==session)return;
        updatePending(session,pending,next.open);
        await wait();
      }
    }catch(error){if(publishing===session)stopHosting(error.message);}
  };

  function stopJoining(){
    const request=joining;joining=null;cancel.hidden=true;
    if(request?.fields)ignore(lanRequest('join-cancel',request.fields,true));
    if(browsing)browsing.key=null;
  }

  function stopBrowsing(){
    const session=browsing;
    browsing=null;
    stopJoining();
    if(session?.scan)ignore(lanRequest('browse-stop',{scan:session.scan},true));
    browsePanel.hidden=true;roomsList.replaceChildren();
  }

  cancel.onclick=()=>{
    stopJoining();
    if(browsing)browsing.notice=true;
    browseStatus.textContent='Beitrittsanfrage zurückgezogen.';
  };

  async function requestJoin(room){
    if(joining||!browsing)return;
    const target={address:room.address,room:room.room},request={fields:null};
    joining=request;cancel.hidden=false;browsing.key=null;browsing.notice=false;
    browseStatus.textContent=`Warte auf die Freigabe für „${room.name}“ …`;
    try{
      const {request:id,guest}=await lanRequest('join-request',{...target,name:get('lan-guest-name').value.trim()});
      request.fields={...target,request:id,guest};
      if(joining!==request){ignore(lanRequest('join-cancel',request.fields,true));return;}
      while(joining===request){
        const {state,ticket}=await lanRequest('join-status',request.fields);
        if(joining!==request)return;
        if(state==='rejected')throw Error('Der Host hat die Beitrittsanfrage abgelehnt.');
        if(state==='approved'){
          const code=lanInviteCode(room.address,ticket);
          joining=null;
          join(code);
          return;
        }
        await wait();
      }
    }catch(error){
      if(joining!==request)return;
      stopJoining();
      if(browsing)browsing.notice=true;
      browseStatus.textContent=error.message;
    }
  }

  function renderRooms(session,rooms,waiting){
    const key=JSON.stringify([rooms,!!joining]);
    if(session.key!==key){
      session.key=key;
      roomsList.replaceChildren();
      for(const room of rooms){
        const row=document.createElement('fieldset'),name=document.createElement('legend');
        name.textContent=room.name;
        const info=document.createElement('p');info.className='dim';
        const phase=room.tick===null?'Neue Partie':`Spielstand · Schritt ${room.tick}`;
        info.textContent=`${modeNames[room.mode]} · ${phase} · ${room.ready}/${room.seats} bereit · ${room.open} frei · ${room.ai} KI · ${room.address}`;
        const rules=document.createElement('p');rules.className='dim';
        rules.textContent=`${climateNames[room.climate]} · ${epochName(room.epoch)} bis ${epochName(room.finalEpoch)}`;
        const apply=button('Beitritt anfragen',()=>void requestJoin(room));
        apply.disabled=!room.open||!!joining;
        row.append(name,info,rules,apply);roomsList.append(row);
      }
    }
    if(joining||session.notice)return;
    browseStatus.textContent=rooms.length?`${rooms.length} LAN-${rooms.length===1?'Raum':'Räume'} gefunden.`:
      waiting?'Suche nach LAN-Räumen …':'Keine sichtbaren LAN-Räume. Die Suche läuft weiter.';
  }

  return {
    availability(lan){canHost=lan.browse===1&&!!lan.host;get('browse').hidden=lan.browse!==1;},
    showHost(){hostPanel.hidden=!canHost;},
    stopHosting,
    reset(){stopHosting();stopBrowsing();hostPanel.hidden=true;},
    async startBrowsing(){
      const session={scan:null,key:null};
      browsing=session;browsePanel.hidden=false;
      browseStatus.textContent='Suche nach LAN-Räumen …';
      try{
        const {scan}=await lanRequest('browse-start');
        if(browsing!==session){ignore(lanRequest('browse-stop',{scan},true));return;}
        session.scan=scan;
        while(browsing===session){
          const {rooms,waiting}=await lanRequest('browse-read',{scan});
          if(browsing!==session)return;
          renderRooms(session,rooms,waiting);
          await wait();
        }
      }catch(error){
        if(browsing!==session)return;
        stopBrowsing();browsePanel.hidden=false;
        browseStatus.textContent=error.message;
      }
    },
  };
}
