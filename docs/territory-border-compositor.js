const DRAW_STRIDE = 8;
const RECORD_WIDTH = 256;

/** Group exact ordered native draw records by the terrain cell they cover. */
export function packTerritoryBorderDraws(draws, width, height) {
  if (!(draws instanceof Float32Array) || draws.length % DRAW_STRIDE ||
      !Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1) {
    throw Error('Invalid territory border draw records');
  }
  const cells = new Uint32Array(width * height * 2);
  const targets = new Int32Array(draws.length / DRAW_STRIDE).fill(-1);
  for (let offset = 0; offset < draws.length; offset += DRAW_STRIDE) {
    const [x, z, cosine, sine, layer, owner, state] = draws.subarray(offset, offset + 7);
    if (!Number.isInteger(x - .5) || !Number.isInteger(z - .5) ||
        !Number.isFinite(cosine) || !Number.isFinite(sine) || !Number.isInteger(layer) ||
        layer < 0 || layer > 29 || !Number.isInteger(owner) || owner < 0 || owner > 16 ||
        ![0, 1, 2].includes(state)) throw Error('Unsupported native territory decal');
    const tileX = Math.floor(x), tileZ = Math.floor(z);
    if (tileX < 0 || tileX >= width || tileZ < 0 || tileZ >= height) continue;
    const tile = tileZ * width + tileX;
    targets[offset / DRAW_STRIDE] = tile;
    cells[tile * 2 + 1]++;
  }

  let recordCount = 0;
  const next = new Uint32Array(width * height);
  for (let tile = 0; tile < width * height; tile++) {
    cells[tile * 2] = recordCount;
    next[tile] = recordCount;
    recordCount += cells[tile * 2 + 1];
  }
  const recordHeight = Math.max(1, Math.ceil(recordCount / RECORD_WIDTH));
  const records = new Float32Array(RECORD_WIDTH * recordHeight * 4);
  for (const [index, tile] of targets.entries()) {
    if (tile < 0) continue;
    const from = index * DRAW_STRIDE;
    const [, , cosine, sine, layer, owner, state] = draws.subarray(from, from + 7);
    const to = next[tile]++ * 4;
    records[to] = cosine;
    records[to + 1] = sine;
    records[to + 2] = layer;
    records[to + 3] = state === 0 ? 0 : owner;
  }
  return {cells, width, height, records, recordWidth:RECORD_WIDTH, recordHeight, recordCount};
}

// Fragment adapter for generated or authored terrain geometry. Native border
// quads occupy one cell at an integral offset, with mirrored full-rect UV.
export const territoryBorderCompositor = `
uniform highp usampler2D uTerritoryBorderCells;
uniform highp sampler2D uTerritoryBorderRecords;
uniform highp sampler2DArray uTerritoryBorderMips;
uniform vec3 uTerritoryBorderTint[17];
uniform float uTerritoryBorderOpacity;
vec2 territoryBorderRotate(vec2 p,vec2 r){
  return vec2(dot(p,r),dot(p,vec2(-r.y,r.x)));
}
vec3 compositeTerritoryBorders(vec3 ground,vec2 world){
  vec2 dx=dFdx(world),dz=dFdy(world);
  ivec2 tile=ivec2(floor(world));
  ivec2 size=textureSize(uTerritoryBorderCells,0);
  if(any(lessThan(tile,ivec2(0)))||any(greaterThanEqual(tile,size)))return ground;
  uvec2 range=texelFetch(uTerritoryBorderCells,tile,0).rg;
  vec2 q=fract(world)-.5;
  uint stride=uint(textureSize(uTerritoryBorderRecords,0).x);
  for(uint i=0u;i<range.y;i++){
    uint index=range.x+i;
    vec4 row=texelFetch(uTerritoryBorderRecords,ivec2(index%stride,index/stride),0);
    vec2 uv=vec2(.5)-territoryBorderRotate(q,row.xy);
    float alpha=textureGrad(uTerritoryBorderMips,vec3(uv,row.z),
      -territoryBorderRotate(dx,row.xy),-territoryBorderRotate(dz,row.xy)).r;
    ground=mix(ground,uTerritoryBorderTint[int(row.w)],alpha*uTerritoryBorderOpacity);
  }
  return ground;
}
`;
