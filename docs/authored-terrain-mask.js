export const VALLEY_TERRAIN_MASK = Object.freeze({
  url: 'local-original/terrain-native/terr_mask.png',
  sha256: 'cd85bb2d2fee4c9f700e66ece7cb7cd2ed2ca755c4a57e70178b9b1dc1a6197c',
});
export const VALLEY_MASK_MIPS = Object.freeze({
  url: 'local-original/valley-terrain/terr-mask-mips.alpha8',
  sha256: 'c1fc05efdde9518b8a4cf0ebdcb72a7cff44c26d441eec37162369c71d714046',
});

export function valleyMaskMipLevels(bytes) {
  if (!(bytes instanceof Uint8Array) || bytes.length !== 87381) return null;
  const levels = [];
  let offset = 0;
  for (let side=256; side>=1; side>>=1) {
    const end = offset + side*side;
    levels.push({side, pixels:bytes.subarray(offset,end)});
    offset = end;
  }
  return levels;
}

// Installed Temperate high-detail definitions for the authenticated Valley map.
const highDetail = new Map([
  [0, [0, 'Grass_Temperate']], [3, [3, 'MountainTop_Temperate']],
  [5, [5, 'GrassPatchy_Temperate']], [6, [6, 'GrassDark_Temperate']],
  [8, [8, 'Mountain01_Temperate']], [16, [15, 'SparseGrass01_Temperate']],
  [17, [16, 'SparseGrass02_Temperate']], [18, [17, 'SparseGrass03_Temperate']],
  [19, [18, 'FloodMeadow_Temperate']], [20, [19, 'HillMeadow_Temperate']],
]);

export function valleyTerrainPasses(geometry, bindings) {
  const {width, height, materials, tints} = geometry;
  const stride = width + 1;
  if (width !== 115 || height !== 71 || materials.length !== stride * (height + 1) ||
      tints.length !== materials.length || tints.some(word => word !== 0xff808080)) {
    throw new Error('Unsupported Valley mask input');
  }
  const definitions = new Map(bindings.map(({materialId, definition}, layer) => {
    const [key, source] = highDetail.get(materialId) ?? [];
    if (key === undefined || source !== definition || layer >= 16) throw new Error('Unknown Valley texture definition');
    return [materialId, {key, layer}];
  }));
  const passes = new Uint8Array(width * height * 4);
  for (let z = 0; z < height; z++) for (let x = 0; x < width; x++) {
    const north = z * stride + x, south = north + stride;
    const corners = [materials[south], materials[south+1], materials[north], materials[north+1]];
    const groups = new Map();
    for (const [bit,id] of corners.entries()) {
      const {key, layer} = definitions.get(id);
      const group = groups.get(key);
      if (group) group.mask |= 1 << bit;
      else groups.set(key, {key, layer, mask: 1 << bit});
    }
    const ordered = [...groups.values()].sort((a,b)=>a.key-b.key);
    for (const [index,{layer,mask}] of ordered.entries()) {
      passes[(z*width+x)*4+index] = layer | (mask << 4);
    }
  }
  return passes;
}

// Origin U,V and X,Z coefficients from the installed affine initializer.
export const valleyMaskBasis = [
  [0,.5,0,0,0,0], [.5,1,-.5,0,0,-.5], [0,1,.5,0,0,-.5], [0,0,0,.5,.5,0],
  [.5,.5,-.5,0,0,.5], [.5,.5,-.5,0,0,-.5], [1,1,-.5,0,0,-.5], [.5,.5,.5,0,0,-.5],
  [0,.5,.5,0,0,.5], [.5,1,.5,0,0,-.5], [0,.5,.5,0,0,-.5], [1,.5,-.5,0,0,-.5],
  [.5,.5,0,-.5,-.5,0], [.5,0,.5,0,0,.5], [1,0,-.5,0,0,.5], [.5,.5,0,0,0,0],
];
const vec3 = values => `vec3(${values.map(value=>Number.isInteger(value)?value.toFixed(1):value).join(',')})`;
const affineU = valleyMaskBasis.map(row=>vec3([row[0],row[2],row[4]])).join(',');
const affineV = valleyMaskBasis.map(row=>vec3([row[1],row[3],row[5]])).join(',');

export const valleyMaskShader = `
uniform highp usampler2D uMaterialPasses;
uniform sampler2D uTerrainMask;
uniform bool uNativeMask;
uniform bool uMaskMipChain;
const vec3 maskU[16]=vec3[16](${affineU});
const vec3 maskV[16]=vec3[16](${affineV});
vec3 terrainPass(uint layer,vec2 uv,vec2 dx,vec2 dz){
 vec3 texel=textureGrad(uMaterials,vec3(uv,float(layer)),dx,dz).rgb;
 return min(texel*(256.0/255.0),1.0);
}
vec3 maskedTerrain(ivec2 tile,vec2 local,vec2 dx,vec2 dz){
 vec2 nearUV=vec2(tile%5)*.4;
 vec2 uv=mix(nearUV,nearUV+.4,local);
 vec2 materialDx=dx*.4,materialDz=dz*.4;
 uvec4 passes=texelFetch(uMaterialPasses,tile,0);
 vec3 colour=terrainPass(passes.x&15u,uv,materialDx,materialDz);
 uint seed=uint(tile.y)*0x3109u+uint(tile.x)*0x1f1u;
 uint mirrors=((seed*0x19660du+0x3c6ef35fu)>>16u)&3u;
 for(int i=1;i<4;i++){
  uint group=passes[i];
  if(group==0u)break;
  int mask=int(group>>4u);
  vec3 at=vec3(1.0,local);
  vec3 basisU=maskU[mask],basisV=maskV[mask];
  vec2 maskUV=vec2(dot(basisU,at),dot(basisV,at))*.5;
  vec2 signUV=vec2(1.0);
  if((mirrors&1u)!=0u){maskUV.x=1.0-maskUV.x;signUV.x=-1.0;}
  if((mirrors&2u)!=0u){maskUV.y=1.0-maskUV.y;signUV.y=-1.0;}
  vec2 maskDx=vec2(dot(basisU.yz,dx),dot(basisV.yz,dx))*.5*signUV;
  vec2 maskDz=vec2(dot(basisU.yz,dz),dot(basisV.yz,dz))*.5*signUV;
  float alpha=uMaskMipChain?textureGrad(uTerrainMask,maskUV,maskDx,maskDz).r:textureLod(uTerrainMask,maskUV,0.0).a;
  colour=mix(colour,terrainPass(group&15u,uv,materialDx,materialDz),alpha);
 }
 return colour;
}
`;
