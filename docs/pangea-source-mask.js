// Ordinary high-detail MaskTerrainBlock groups the four terrain corners by
// source texture-definition key. Keep equal definitions together even when
// the caller represents them with separate terrain IDs.
export function packedTerrainPasses(raster,side,definitions){
  const keys=new Map(definitions.map(({id,key})=>[id,key]));
  const stride=side+1;
  const packed=new Uint16Array(side*side*4);
  for(let z=0;z<side;z++)for(let x=0;x<side;x++){
    const north=z*stride+x,south=north+stride;
    const corners=[raster[south],raster[south+1],raster[north],raster[north+1]];
    const groups=new Map();
    for(const [bit,id] of corners.entries()){
      const key=keys.get(id);
      if(key===undefined)throw new Error('Pangea Texturschlüssel');
      const group=groups.get(key);
      if(group)group.mask|=1<<bit;
      else groups.set(key,{id,mask:1<<bit});
    }
    const ordered=[...groups.keys()].sort((a,b)=>a-b);
    const at=(z*side+x)*4;
    for(const [layer,key] of ordered.entries()){
      const {id,mask}=groups.get(key);
      packed[at+layer]=id|(mask<<5);
    }
  }
  return packed;
}

export const sourceMaskShader=`
uniform highp usampler2D uPasses;
uniform sampler2D uSourceMask;
uniform vec3 uMaskU[16],uMaskV[16];

uint maskMirrors(ivec2 tile){
 uint seed=uint(tile.y)*0x3109u+uint(tile.x)*0x1f1u;
 return ((seed*0x19660du+0x3c6ef35fu)>>16u)&3u;
}
float maskAlpha(int mask,vec2 local,uint mirrors){
 vec3 at=vec3(1.0,local);
 vec2 uv=vec2(dot(uMaskU[mask],at),dot(uMaskV[mask],at))*.5;
 if((mirrors&1u)!=0u)uv.x=1.0-uv.x;
 if((mirrors&2u)!=0u)uv.y=1.0-uv.y;
 return textureLod(uSourceMask,uv,0.0).a;
}
vec2 sourceUv(ivec2 tile,vec2 local){
 vec2 near=vec2(tile%5)*.4;
 return mix(near,near+.4,local);
}
int cornerId(ivec2 p){
 return int(texelFetch(uCorners,p,0).r*255.0+.5);
}
vec3 sampleMaterial(int id,vec2 uv,vec2 dx,vec2 dz){
 ivec2 layers=uSlots[id];
 vec3 first=textureGrad(uMaterials,vec3(uv,float(layers.x)),dx,dz).rgb;
 if(layers.x==layers.y)return first;
 return mix(first,textureGrad(uMaterials,vec3(uv,float(layers.y)),dx,dz).rgb,uPhase);
}
vec3 edgeColour(ivec2 a,ivec2 b,float along,bool vertical,vec2 uv,vec2 dx,vec2 dz){
 int first=cornerId(a),second=cornerId(b);
 vec3 colour=sampleMaterial(first,uv,dx,dz);
 if(first==second)return colour;
 float blend=vertical?maskAlpha(1,vec2(0.0,along),maskMirrors(a)):
  maskAlpha(8,vec2(along,0.0),maskMirrors(a));
 return mix(colour,sampleMaterial(second,uv,dx,dz),blend);
}
vec3 sourceTerrain(ivec2 tile,vec2 local,vec2 dx,vec2 dz){
 vec2 uv=sourceUv(tile,local);
 uvec4 passes=texelFetch(uPasses,tile,0);
 vec3 colour=sampleMaterial(int(passes.x&31u),uv,dx,dz);
 uint mirrors=maskMirrors(tile);
 for(int layer=1;layer<4;layer++){
  uint group=passes[layer];
  if(group==0u)break;
  int mask=int(group>>5u),id=int(group&31u);
  colour=mix(colour,sampleMaterial(id,uv,dx,dz),maskAlpha(mask,local,mirrors));
 }

 // Opposing tiles can project different interiors of the source mask at
 // their shared edge. Agree on that edge using its own stable source mask.
 vec2 distanceToEdge=min(local,1.0-local);
 if(distanceToEdge.y<.08){
  ivec2 a=tile+ivec2(0,local.y>.5?1:0);
  vec3 edge=edgeColour(a,a+ivec2(1,0),local.x,false,uv,dx,dz);
  colour=mix(edge,colour,smoothstep(0.0,.08,distanceToEdge.y));
 }
 if(distanceToEdge.x<.08){
  ivec2 a=tile+ivec2(local.x>.5?1:0,0);
  vec3 edge=edgeColour(a,a+ivec2(0,1),local.y,true,uv,dx,dz);
  colour=mix(edge,colour,smoothstep(0.0,.08,distanceToEdge.x));
 }
 return colour;
}`;
