const sourceArchive='8f0f6028cda68dcaa395fffda9973f83a7f24ea22f2a12361cafa1fc392bcec4';

export function prepareAuthoredRectangularView(wasm, bytes) {
  if(!(bytes instanceof Uint8Array)||typeof wasm.native_rect_alloc!=='function')return null;
  const input=wasm.native_rect_alloc(bytes.length);
  if(!input)return null;
  new Uint8Array(wasm.memory.buffer,input,bytes.length).set(bytes);
  if(wasm.native_rect_prepare()!==1)return null;
  const [width,height,territories,startCount,cliffCount,archivedUnits,archivedForest]=Array.from({length:7},
    (_,field)=>wasm.native_rect_info(field));
  if(width!==100||height!==300||territories!==43||startCount!==4||cliffCount!==453){
    throw Error('Unerwartete Daten der Rechteckkarte.');
  }
  const field=(kind,slot,length)=>{
    const ptr=wasm.native_rect_ptr(slot);
    if(!ptr)throw Error('Daten der Rechteckkarte fehlen.');
    return new kind(wasm.memory.buffer,ptr,length).slice();
  };
  const sourceArchiveSha256=Array.from(field(Uint8Array,4,32),value=>value.toString(16).padStart(2,'0')).join('');
  if(sourceArchiveSha256!==sourceArchive)throw Error('Die Rechteckkarte stimmt nicht mit dem Originalarchiv überein.');
  const corners=(width+1)*(height+1),cells=width*height;
  return {width,height,sourceArchiveSha256,cliffCount,archivedUnits,archivedForest,
    elevations:field(Float32Array,0,corners),materials:field(Uint16Array,1,corners),
    tints:field(Uint32Array,2,corners),patterns:field(Uint32Array,3,cells),
    territoryIds:field(Int32Array,5,cells),compactTerritories:field(Uint8Array,6,cells),
    adjacency:Array.from({length:territories},(_,region)=>wasm.native_rect_adjacent(region)),
    starts:Array.from({length:startCount},(_,slot)=>Array.from({length:3},(_,part)=>wasm.native_rect_start(slot,part)))};
}
