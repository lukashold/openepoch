// Visibility and resource availability use the same bytes as the main world view.
export function paintFullmapLayers(wasm, size, terrain, options, rgba) {
  const cells = size*size;
  const view = (ptr,length) => new Uint8Array(wasm.memory.buffer,ptr,length);
  const fog = view(wasm.fog_ptr(),cells);
  const resources = [];
  const usable = options.resources ? Array.from({length:9},(_,i)=>!!wasm.resource_active(options.local,i)) : [];
  rgba.fill(0);
  for (let tile=0; tile<cells; tile++) {
    const o = tile*4, visibility = fog[tile];
    if (!visibility) {
      rgba[o+3] = 255;
      continue;
    }
    if (options.resources) {
      if (terrain[tile] === 4) {
        rgba[o] = 23; rgba[o+1] = 64; rgba[o+2] = 32; rgba[o+3] = Math.max(rgba[o+3],114);
      } else {
        const kind = wasm.resource_kind(tile);
        if (kind && (kind===10 || usable[kind-1])) resources.push({kind,x:tile%size+.5,y:Math.floor(tile/size)+.5});
      }
    }
    if (visibility !== 255) {
      const alpha = rgba[o+3];
      for (let c=0; c<3; c++) rgba[o+c] *= alpha/(alpha+255);
      rgba[o+3] = (alpha+255)/2;
    }
  }
  return resources;
}
