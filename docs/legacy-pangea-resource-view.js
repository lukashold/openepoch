// HNR1 keeps its original engine and peer identity. Only the local view changes.
const deposits = [
  {kind:6, source:106, end:6, marker:[160,180,190]},
  {kind:3, source:102, end:9, marker:[150,150,165]},
  {kind:7, source:107, end:12, marker:[210,195,170]},
];

const rgb = word => [word >>> 16 & 255, word >>> 8 & 255, word & 255];
const shade = (channel,tint,weight,fog) => (((channel*(256-weight)+tint*weight)>>8)*fog)>>8;

export function createLegacyPangeaResourceView(wasm, assetMapping, side) {
  const rows = deposits.map(deposit => {
    const asset = assetMapping.find(row => row.kind===1 && row.sourceId===deposit.source);
    if (!asset) throw Error('Pangea resource render binding is missing');
    return {...deposit, renderId:asset.renderId};
  });
  const byKind = [];
  for (const row of rows) byKind[row.kind] = row;
  const palette = Array.from({length:10}, (_,owner) => rgb(wasm.huge10_team_rgb(3,owner)));
  const neutral = [0,0,0];
  const output = new Uint8ClampedArray(side*side*4);
  let filtered;

  function ranges(original, owner) {
    const epoch = wasm.stat(owner,0);
    if (!rows.some(row => epoch>=row.end && original[row.renderId*2+1])) return original;
    if (filtered?.length!==original.length) filtered = new Uint32Array(original.length);
    filtered.set(original);
    for (const row of rows) if (epoch>=row.end) filtered[row.renderId*2+1] = 0;
    return filtered;
  }

  function minimap(owner) {
    const mini = wasm.minimap();
    const epoch = wasm.stat(owner,0);
    const original = new Uint8ClampedArray(wasm.memory.buffer,mini,output.length);
    if (epoch<rows[0].end) return original;
    output.set(original);
    const map = new Uint8Array(wasm.memory.buffer,wasm.map_ptr(),output.length);
    const visibility = new Uint8Array(wasm.memory.buffer,wasm.fog_ptr(),side*side);
    for (let tile=0; tile<visibility.length; tile++) {
      const visible = visibility[tile];
      if (!visible) continue;
      const row = byKind[wasm.resource_kind(tile)];
      if (!row || epoch<row.end) continue;
      const x = tile%side, y = Math.floor(tile/side);
      const territory = wasm.territory_at(x+.5,y+.5);
      const tint = palette[wasm.territory_owner(territory)];
      const edge = tint && (
        x+1<side && wasm.territory_at(x+1.5,y+.5)!==territory ||
        y+1<side && wasm.territory_at(x+.5,y+1.5)!==territory
      );
      const weight = tint ? edge ? 150 : 28 : 0;
      const color = tint ?? neutral;
      const fog = visible===255 ? 256 : 130;
      const pixel = tile*4;
      // Unit and building colors are painted over the deposit in the engine.
      if (!row.marker.every((channel,i) => original[pixel+i]===shade(channel,color[i],weight,fog))) continue;
      for (let i=0; i<3; i++) output[pixel+i] = shade(map[pixel+i],color[i],weight,fog);
    }
    return output;
  }

  return Object.freeze({ranges,minimap});
}
