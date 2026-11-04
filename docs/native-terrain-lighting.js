// Isolated transcription of proved MaskRenderer operations. Not imported by5.
// Block extent/UV/filter operations are source-bound. Native shadow-list
// emitter geometry and original DEM/cliff inputs remain separate bindings.
const f=Math.fround;
export const SOURCE_SELF_SHADOW_WEIGHTS=Object.freeze([[1,2,1],[2,4,2],[1,2,1]]);
export function nativeInterpolateNormal(corners,fx,fz){
 //006ED6B0 interpolates stored vertex normals on diagonal00→11, then
 //004EC580 normalizes with epsilon0. Zero vectors remain zero.
 const [n00,n10,n01,n11]=corners;
 const value=fx<=fz?n00.map((n,i)=>n*(1-fz)+n01[i]*(fz-fx)+n11[i]*fx):n00.map((n,i)=>n*(1-fx)+n10[i]*(fx-fz)+n11[i]*fz);
 const length=Math.hypot(...value);return length>0?value.map(x=>f(x/length)):value;
}
export function nativeSelfVisibility(occluded,size,x,z){
 if(!Number.isInteger(size)||size<1||occluded.length!==size*size||!Number.isInteger(x)||!Number.isInteger(z))throw new RangeError('Invalid source shadow grid');
 const at=(x,z)=>occluded[Math.max(0,Math.min(size-1,z))*size+Math.max(0,Math.min(size-1,x))]?1:0;
 let total=0;
 for(let dz=-1;dz<=1;dz++)for(let dx=-1;dx<=1;dx++)total+=at(x+dx,z+dz)*SOURCE_SELF_SHADOW_WEIGHTS[dz+1][dx+1];
 //006EDAE0: exact integer centre4/axes2/corners1, then1-total*.0625.
 return 1-total/16;
}
export function nativeLightmapByte(normal,towardsSun,selfVisibility=1){
 //006EE070 multiplies the signed normal dot by integer visibilityByte.
 //The source sampler supplies a normalized normal; do not silently invent one.
 const visibilityByte=Math.trunc(selfVisibility*255),dot=normal.reduce((sum,n,i)=>sum+n*towardsSun[i],0);
 return Math.max(0,Math.trunc(dot*visibilityByte))&255;
}
const pack=value=>Math.trunc(Math.min(1,value)*255)&255;
export function nativeTerrainCoefficients(lighting){
 const {diffuseScale,ambientScale,shadowScale}=lighting.terrain;
 //sun/ambient have already passed through selected native light Dimmer1.5.
 const diffuse=lighting.sun.map(x=>Math.min(1,f(x*diffuseScale)));
 const ambient=lighting.ambient.map(x=>Math.min(1,f(x*2*ambientScale)));
 //006F41D0 stores separate raw floats and truncates their255-scaled values to
 //D3DCOLOR. Its shadow ratio uses raw floats, not the quantized colours.
 const diffuseBytes=diffuse.map(pack),ambientBytes=ambient.map(pack);
 const shadowBytes=ambient.map((a,i)=>{
  const denominator=2*lighting.direction[1]*diffuse[i]+a;
  return denominator===0?null:Math.trunc(a*shadowScale*255/denominator)&255;
 });
 return {diffuse,ambient,diffuseBytes,ambientBytes,shadowBytes};
}
export function nativeLightmapResolve(baseTexture,lightmapByte,coefficients,diffuseRaster=coefficients.ambientBytes.map(x=>x/255)){
 //006F41D0: stage0 MODULATE2X(texture,textureFactor), stage1 ADD(current,
 //diffuse). SourceBlend DESTCOLOR + DestinationBlend SRCCOLOR doubles their
 //product with the already drawn ground.006F3580 gives00942C40→00943080 an
 //empty effect state, disabling previous lights;006F41D0 then supplies white
 //ambient material, zero emission and the packed ambient render-state colour.
 //Therefore the source diffuse raster colour is ambientBytes/255.
 return baseTexture.slice(0,3).map((base,i)=>{
  const stage0=Math.min(1,2*(lightmapByte/255)*(coefficients.diffuseBytes[i]/255));
  const stage1=Math.min(1,stage0+diffuseRaster[i]);
  return Math.min(1,2*base*stage1);
 });
}
export function nativeHighShadowResolve(baseTexture,lightmapByte,coefficients,stencil){
 //006F6050 marks alpha>=128 coverage, STENCILPASS=INCR, writeMask0x3F.
 //006F41D0 draws lightmap for lower6bits==0, then changes TFACTOR to0
 //and redraws where lower6bits!=0. Ambient remains; overlap counts once.
 return nativeLightmapResolve(baseTexture,(stencil&63)===0?lightmapByte:0,coefficients);
}
