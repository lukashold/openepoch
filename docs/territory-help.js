// Claim status comes from the simulation; this module only presents it.
export function territoryClaimMessage(area,{building=false}={}){
  if(!area||area.known===false)return 'Erkunde dieses Gebiet zuerst.';
  switch(area.claimStatus){
    case 1:return 'Dieses Gebiet gehört dir.';
    case 2:return 'Das Stadtzentrum wird noch gebaut.';
    case 3:return 'Abgelegenes Gebiet: Zusätzlich zum Stadtzentrum ist eine eigene Festung erforderlich.';
    case 4:return 'Die Festung wird noch gebaut; der Gebietsanspruch wartet.';
    case 5:return 'Erobere den Hügel, um dieses Gebiet zu übernehmen.';
  }
  if(area.owner!==255)return 'Dieses Gebiet gehört einem anderen Spieler.';
  if(area.adjacent)return 'Angrenzendes freies Gebiet: Ein eigenes Stadtzentrum genügt für den Gebietsanspruch.';
  return building?'Abgelegenes freies Gebiet: Stadtzentrum und eigene Festung für den Gebietsanspruch bauen.':'Abgelegenes freies Gebiet: Für den Gebietsanspruch brauchst du ein Stadtzentrum und eine eigene Festung.';
}
