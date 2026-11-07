// Exchange only WebRTC descriptions. Game traffic stays on the data channel.
const PATH='/__openepoch__/lan';
const PREFIX='OE-L1.';
const TICKET=/^[A-Za-z0-9_-]{32}$/;
const IPV4=/^\d{1,3}(?:\.\d{1,3}){3}$/;
const LOCAL_PAGE=location.protocol==='http:'&&['127.0.0.1','localhost'].includes(location.hostname);
const messages={
  'private-address':'Die Einladung benötigt eine private LAN-Adresse.',
  'host-unreachable':'Der LAN-Host ist nicht erreichbar. Prüfe das lokale Netz und den Spielstarter des Hosts.',
  'host-disabled':'Der lokale Spielstarter hat LAN-Einladungen nicht aktiviert.',
  'ticket-expired':'Die LAN-Einladung ist abgelaufen. Lass dir eine neue Einladung geben.',
  'seat-claimed':'Dieser LAN-Platz wurde schon beansprucht. Lass dir eine neue Einladung geben.',
  'room-limit':'Der Spielstarter hat bereits zu viele offene LAN-Einladungen.',
  'browse-unavailable':'Die LAN-Raumsuche konnte den lokalen UDP-Port 8127 nicht öffnen.',
  'browse-in-use':'In einem anderen Fenster läuft bereits eine LAN-Raumsuche.',
  'browse-expired':'Die LAN-Raumsuche wurde beendet.',
  'room-already-listed':'Dieser Spielstarter zeigt bereits einen LAN-Raum an.',
  'room-expired':'Der LAN-Raum ist nicht mehr sichtbar.',
  'room-full':'In diesem LAN-Raum ist gerade kein Platz frei.',
  'room-name':'Der Name muss aus 1 bis 40 sichtbaren Zeichen bestehen.',
  'join-expired':'Die Beitrittsanfrage ist nicht mehr gültig.',
};

export async function lanRequest(action, fields={}, keepalive=false) {
  if(!LOCAL_PAGE)throw Error('LAN-Verbindungen benötigen den lokalen OpenEpoch-Spielstarter.');
  let response;
  const controller=new AbortController();
  const timer=setTimeout(()=>controller.abort(),5000);
  try{
    response=await fetch(PATH,{method:'POST',headers:{'Content-Type':'application/json'},
      body:JSON.stringify({action,...fields}),cache:'no-store',credentials:'same-origin',keepalive,signal:controller.signal});
    const result=await response.json();
    if(response.ok)return result;
    throw Error(messages[result.error]??'Die LAN-Einladung konnte nicht verarbeitet werden.');
  }catch(error){
    if(!response||error instanceof SyntaxError||error.name==='AbortError')
      throw Error('LAN-Verbindungen benötigen den lokalen OpenEpoch-Spielstarter.');
    throw error;
  }finally{clearTimeout(timer);}
}

export const isLanInvite=code=>code.trim().startsWith(PREFIX);

export async function lanStatus(){
  try{return await lanRequest('status');}
  catch{return {available:false,host:null};}
}

export async function publishLanInvite(address,assignment,offer){
  if(!IPV4.test(address))throw Error(messages['host-disabled']);
  const authority=await lanRequest('publish',{...assignment,offer});
  const {ticket,owner}=authority;
  if(!TICKET.test(ticket)||!TICKET.test(owner))throw Error('Der Spielstarter lieferte keine gültige Einladung.');
  let closed=false;
  return {
    code:lanInviteCode(address,ticket),
    approve:(roomOwner,request)=>lanRequest('room-approve',{owner:roomOwner,request,ticket,ticketOwner:owner}),
    async answer(){
      while(!closed){
        const {answer}=await lanRequest('read',authority);
        if(closed||answer)return closed?null:answer;
        await new Promise(done=>setTimeout(done,650));
      }
      return null;
    },
    close(){
      if(closed)return;
      closed=true;
      void lanRequest('close',authority,true).catch(()=>{});
    },
  };
}

export function lanInviteCode(address,ticket){
  if(!IPV4.test(address)||!TICKET.test(ticket))throw Error('Ungültige LAN-Einladung.');
  return `${PREFIX}${address}:${ticket}`;
}

export async function claimLanInvite(code){
  const fields=code.trim().slice(PREFIX.length).split(':');
  const [address,ticket]=fields;
  if(fields.length!==2||!IPV4.test(address)||!TICKET.test(ticket))throw Error('Ungültige LAN-Einladung.');
  const {offer,claim,room,seat,seats}=await lanRequest('claim',{address,ticket});
  const authority={address,ticket,claim};
  return {sdp:offer,room,seat,seats,
    answer:answer=>lanRequest('answer',{...authority,answer}),
    release:()=>{void lanRequest('release',authority,true).catch(()=>{});},
  };
}
