// Source UnitType overlay records bound to the existing typed render stream.
export function fullmapActorCatalog(manifest) {
  if (manifest?.format !== 'ee2-source-fullmap-actor-overlays-v1' || manifest.definitions !== 81) {
    throw new Error('Unsupported full-map actor overlay catalog');
  }
  const definitions = Array.from({length:manifest.definitions}, () => Array(15));
  for (const row of manifest.rows) {
    const {id, epoch, until, overlay, size, shape} = row;
    if (!Number.isInteger(id) || !definitions[id] || !Number.isInteger(epoch) || !Number.isInteger(until)
      || epoch < 0 || epoch >= until || until > 15 || !/^spr_fullmap_[a-z0-9_]+$/.test(overlay?.sprite)) {
      throw new Error('Invalid full-map actor overlay row');
    }
    if (!Array.isArray(size) || size.length !== 2 || !size.every(value=>Number.isFinite(value)&&value>0)
      || !['ellipse','rectangle'].includes(shape)) throw new Error('Invalid full-map footprint');
    const factor = shape==='rectangle' ? Math.fround(Math.SQRT2) : 1;
    const axes = Object.freeze(size.map(value=>Math.fround(Math.fround(value)*factor)));
    const binding = Object.freeze({...overlay, axes, sourceUnit:row.sourceUnit});
    for (let age=epoch; age<until; age++) {
      if (definitions[id][age]) throw new Error('Overlapping full-map actor overlay rows');
      definitions[id][age] = binding;
    }
  }
  return Object.freeze({binding:(def,epoch)=>definitions[def]?.[epoch] ?? null});
}

export function fullmapActorSize(overlay, width, height, worldWidth, worldHeight) {
  const scale = Math.fround(overlay.scale || 1);
  const w = Math.fround(Math.fround(overlay.axes[0]*scale)*width/worldWidth);
  const h = Math.fround(Math.fround(overlay.axes[1]*scale)*height/worldHeight);
  return w<2 || h<2 ? [2,2] : [w,h];
}

export function readFullmapActors(wasm, metadata, catalog, {
  width, height, showUnits=true, showBuildings=true, hillDefinition, fishingId,
}) {
  if (!showUnits && !showBuildings) return [];
  const count = wasm.render(1,0,0,width,height,-1,0,0);
  const instances = new Float32Array(wasm.memory.buffer,wasm.inst_ptr(),count*12);
  const {actors} = metadata.snapshot({count,instances});
  const epochs = [0,1,2,3].map(seat=>wasm.stat(seat,0));
  const hillOverlay = {sprite:'spr_fullmap_building',axes:[Math.fround(2*Math.SQRT2),Math.fround(2*Math.SQRT2)]};
  const fishingOverlay = {sprite:'spr_fullmap_boat',axes:[.75,.5]};
  const overlays = [];
  for (const actor of actors) {
    const hill = actor.def===hillDefinition;
    if (actor.hpRatio<=0 || (!hill&&actor.team>=epochs.length)) continue;
    const building = !!(wasm.definition_flags(actor.def) & 1);
    if (!(building ? showBuildings : showUnits)) continue;
    const overlay = hill ? hillOverlay : actor.def===fishingId ? fishingOverlay : catalog.binding(actor.def,epochs[actor.team]);
    if (!overlay) continue;
    overlays.push({handle:actor.handle,owner:actor.team,building,def:actor.def,x:actor.simX,y:actor.simY,
      yaw:actor.renderYaw,built:actor.built,overlay});
  }
  return overlays;
}
