export const PLACEMENT_ERRORS=Object.freeze([
  '',
  'Die Baufläche ist durch Gelände, Rohstoffe oder Gebäude belegt.',
  'Hier darfst du mit den aktuellen Gebietsrechten nicht bauen.',
  'Erkunde die gesamte Baufläche zuerst.',
  'Für dieses Gebäude fehlen Rohstoffe.',
  'In diesem Gebiet gibt es bereits ein Stadtzentrum. Pro Gebiet ist eines möglich.',
  'Wähle mindestens einen eigenen Bürger zum Bauen aus.',
  'Kein Landweg für die gewählten Bürger. Öffne einen Zugang oder setze sie beim Bauplatz ab.',
  'Dieses Gebäude ist in deiner Epoche noch nicht verfügbar.',
  'Das Einheitenlimit ist erreicht.',
  'In einem Gebiet sind höchstens zwei Festungen möglich.',
  'Der anfängliche Waffenstillstand verbietet den Bau an dieser Stelle.',
  'Am Krisenherd darf nur eine Festung direkt am Marker gebaut werden.',
  'In einem Gebiet sind höchstens sechs Häuser möglich.'
]);

export function placementMessage(error,id){
  if(error===0)return 'Bauplatz möglich · Linksklick zum Bauen';
  if(error===1&&id===5)return 'Der Hafen braucht eine freie Wasserfläche direkt am begehbaren Ufer.';
  return PLACEMENT_ERRORS[error]||PLACEMENT_ERRORS[1];
}
export function nextBuildingTurn(turn,direction,fine=false){return (turn+(direction<0?-1:1)*(fine?1:2)+8)%8;}

export function createBuildingControls({document:doc=globalThis.document,rotate,cancel,automatic}={}){
  const panel=doc.getElementById('building-controls'),heading=doc.getElementById('building-name'),
    angle=doc.getElementById('building-angle'),status=doc.getElementById('building-status');
  doc.getElementById('building-left').onclick=event=>rotate(-1,event.shiftKey);
  doc.getElementById('building-right').onclick=event=>rotate(1,event.shiftKey);
  doc.getElementById('building-cancel').onclick=cancel;
  const auto=doc.getElementById('building-auto');auto.onclick=automatic;
  return {
    hide(){panel.hidden=true;},
    update({name,id,turn,error,size,width=size,height=size,manual,note=''}){
      panel.hidden=false;heading.textContent=name;angle.textContent=`${turn*45}° · ${width} × ${height} Felder`;
      auto.hidden=id!==5;auto.textContent=manual?'Küste: manuell':'Küste: automatisch';
      const text=placementMessage(error,id)+(note?` · ${note}`:'');if(status.textContent!==text)status.textContent=text;
      status.dataset.valid=String(error===0);
    }
  };
}
