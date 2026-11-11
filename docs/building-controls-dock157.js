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
  'In einem Gebiet sind höchstens sechs Häuser möglich.',
  'In einem Gebiet ist nur ein Gebäude dieses Typs möglich.'
]);

export function placementMessage(error,id){
  if(error===0)return 'Bauplatz möglich · Linksklick zum Bauen';
  if(error===1&&id===5)return 'Der Hafen braucht eine freie Wasserfläche direkt am begehbaren Ufer.';
  return PLACEMENT_ERRORS[error]||PLACEMENT_ERRORS[1];
}
export function nextBuildingTurn(turn,direction,fine=false){return (turn+(direction<0?-1:1)*(fine?1:2)+8)%8;}

export function createBuildingControls({document:doc=globalThis.document,rotate,cancel,automatic}={}){
  const panel=doc.getElementById('building-controls'),heading=doc.getElementById('building-name'),
    angle=doc.getElementById('building-angle'),status=doc.getElementById('building-status'),
    left=doc.getElementById('building-left'),right=doc.getElementById('building-right'),hint=panel.querySelector('small');
  left.onclick=event=>rotate(-1,event.shiftKey);
  right.onclick=event=>rotate(1,event.shiftKey);
  doc.getElementById('building-cancel').onclick=cancel;
  const auto=doc.getElementById('building-auto');auto.onclick=automatic;
  let nativeFarm=false;
  return {
    hide(){panel.hidden=true;},
    update({name,id,turn,yaw,error,size,width=size,height=size,manual,automaticDock=false,note=''}){
      panel.hidden=false;heading.textContent=name;
      const continuous=Number.isFinite(yaw);
      const farmRotation=continuous&&!automaticDock;
      if(farmRotation!==nativeFarm){
        nativeFarm=farmRotation;
        const step=nativeFarm?'45°':'90°',fine=nativeFarm?'11,25°':'45°';
        left.textContent=`↶${step}`;right.textContent=`${step}↷`;
        left.title=`${step} nach links · mit Umschalt ${fine}`;
        right.title=`${step} nach rechts · mit Umschalt ${fine}`;
        hint.textContent=nativeFarm?'Strg: 45° · Alt+Strg: 11,25° · Esc: abbrechen':
          'Strg / R / Komma / Punkt: 90° · mit Umschalt: 45° · Umschalt+Mausrad: 45° · Esc: abbrechen';
      }
      const degrees=continuous?((Math.round(yaw*18000/Math.PI)%36000+36000)%36000/100).toLocaleString('de-DE'):turn*45;
      angle.textContent=automaticDock?(continuous?`${degrees}° · Küstenausrichtung`:'Küstenausrichtung'):`${degrees}° · ${width} × ${height} Felder`;
      auto.hidden=id!==5;auto.textContent=manual?'Küste: manuell':'Küste: automatisch';
      const text=placementMessage(error,id)+(note?` · ${note}`:'');if(status.textContent!==text)status.textContent=text;
      status.dataset.valid=String(error===0);
    }
  };
}
