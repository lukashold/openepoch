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
};

async function request(action, fields={}, keepalive=false) {
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
  try{return await request('status');}
  catch{return {available:false,host:null};}
}

export async function publishLanInvite(address,assignment,offer){
  if(!IPV4.test(address))throw Error(messages['host-disabled']);
  const authority=await request('publish',{...assignment,offer});
  const {ticket,owner}=authority;
  if(!TICKET.test(ticket)||!TICKET.test(owner))throw Error('Der Spielstarter lieferte keine gültige Einladung.');
  let closed=false;
  return {
    code:`${PREFIX}${address}:${ticket}`,
    async answer(){
      while(!closed){
        const {answer}=await request('read',authority);
        if(closed||answer)return closed?null:answer;
        await new Promise(done=>setTimeout(done,650));
      }
      return null;
    },
    close(){
      if(closed)return;
      closed=true;
      void request('close',authority,true).catch(()=>{});
    },
  };
}

export async function claimLanInvite(code){
  const fields=code.trim().slice(PREFIX.length).split(':');
  const [address,ticket]=fields;
  if(fields.length!==2||!IPV4.test(address)||!TICKET.test(ticket))throw Error('Ungültige LAN-Einladung.');
  const {offer,claim,room,seat,seats}=await request('claim',{address,ticket});
  const authority={address,ticket,claim};
  return {sdp:offer,room,seat,seats,
    answer:answer=>request('answer',{...authority,answer}),
    release:()=>{void request('release',authority,true).catch(()=>{});},
  };
}
