import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {buildTerrainFields,loadOriginalTerrain,TERRAIN_SHADERS,WATER_SHADERS} from '../docs/original-terrain.js';

let checks=0;
const check=(value,message)=>{assert.ok(value,message);checks++;};
const size=12,heights=new Float32Array((size+1)**2).fill(.4);
for(const [id,channel] of [[0,3],[1,3],[2,3],[3,0],[4,1],[5,2]]){
 const result=buildTerrainFields({heights,types:new Uint8Array(size*size).fill(id),size});
 check(result.classification==='simulation-terrain-ids','Explicit terrain IDs');
 for(let i=0;i<size*size;i++)for(let c=0;c<4;c++)check(result.weights[i*4+c]===(c===channel?255:0),`Uniform class${id} channel${c}`);
}
const split=new Uint8Array(size*size).fill(3);for(let y=0;y<size;y++)for(let x=size/2;x<size;x++)split[y*size+x]=4;
const transition=buildTerrainFields({heights,types:split,size});
for(let i=0;i<size*size;i++){
 const sum=transition.weights.slice(i*4,i*4+4).reduce((a,b)=>a+b,0);
 check(Math.abs(sum-255)<=2,'Normalized transition weights');
}
const row=Array.from({length:size},(_,x)=>transition.weights[(5*size+x)*4+1]);
for(let i=1;i<row.length;i++)check(row[i]>=row[i-1],'Monotonic forest edge');
check(row.some(x=>x>0&&x<255),'Continuous blend across a categorical boundary');
check(row[0]===0&&row.at(-1)===255,'Far material interiors preserved');
const before=transition.types[0];split[0]=5;check(transition.types[0]===before,'Source types copied');
for(const args of [{heights:heights.subarray(1),size},{heights,types:new Uint8Array(1),size},{heights,types:new Uint8Array(size*size).fill(6),size},{heights:new Float32Array((size+1)**2).fill(NaN),size}]){
 assert.throws(()=>buildTerrainFields(args),RangeError);checks++;
}
check(buildTerrainFields({heights,size}).classification==='height-adapter','Fallback provenance is explicit');

class GL {
 constructor(){
  Object.assign(this,{TEXTURE0:33984,TEXTURE_2D:3553,TEXTURE_2D_ARRAY:35866,RGBA:6408,RGBA8:32856,R32F:33326,RED:6403,FLOAT:5126,UNSIGNED_BYTE:5121,
   TEXTURE_MIN_FILTER:10241,TEXTURE_MAG_FILTER:10240,TEXTURE_WRAP_S:10242,TEXTURE_WRAP_T:10243,LINEAR:9729,NEAREST:9728,
   LINEAR_MIPMAP_LINEAR:9987,REPEAT:10497,CLAMP_TO_EDGE:33071,VERTEX_SHADER:35633,FRAGMENT_SHADER:35632,
   COMPILE_STATUS:35713,LINK_STATUS:35714,TRIANGLES:4,TRIANGLE_STRIP:5,UNSIGNED_INT:5125,UNPACK_FLIP_Y_WEBGL:37440});
  this.unit=0;this.bindings=new Map([[1,{sentinel:'legacy map texture'}]]);this.textures=[];this.draws=[];this.uniforms=new Map();
 }
 createProgram(){return {shaders:[]};}createShader(type){return {type};}shaderSource(s,source){s.source=source;}compileShader(){}getShaderParameter(){return true;}getShaderInfoLog(){return '';}
 attachShader(p,s){p.shaders.push(s);}deleteShader(){}linkProgram(){}getProgramParameter(){return true;}getProgramInfoLog(){return '';}
 getUniformLocation(p,name){return name;}useProgram(p){this.program=p;}uniform1i(l,v){this.uniforms.set(l,v);}uniform1f(l,v){this.uniforms.set(l,v);}uniform2fv(l,v){this.uniforms.set(l,Array.from(v));}uniform3fv(l,v){this.uniforms.set(l,Array.from(v));}uniform4fv(l,v){this.uniforms.set(l,Array.from(v));}uniformMatrix4fv(l,t,v){this.uniforms.set(l,Array.from(v));}
 createTexture(){const t={};this.textures.push(t);return t;}activeTexture(unit){this.unit=unit-this.TEXTURE0;}bindTexture(target,t){check([this.TEXTURE_2D,this.TEXTURE_2D_ARRAY].includes(target),'Texture target valid');this.bindings.set(this.unit,t);}
 texImage2D(...args){const t=this.bindings.get(this.unit);if(args.length===6){t.width=args[5].width;t.height=args[5].height;t.image=args[5].url;}else{t.width=args[3];t.height=args[4];t.data=new args[8].constructor(args[8]);t.internal=args[2];}}
 texStorage3D(target,levels,internal,width,height,depth){Object.assign(this.bindings.get(this.unit),{target,levels,internal,width,height,depth,images:[]});}
 texSubImage3D(target,level,x,y,z,width,height,depth,format,type,image){const t=this.bindings.get(this.unit);check(image.width===width&&image.height===height,'Monthly pixels are not rescaled');t.images[z]=image.url;}
 pixelStorei(){}texParameteri(){}generateMipmap(){}getExtension(){return null;}bindVertexArray(vao){this.vao=vao;}
 drawElements(...args){this.draws.push({kind:'terrain',args,uniforms:new Map(this.uniforms),bindings:new Map(this.bindings)});}
 drawArrays(...args){this.draws.push({kind:'water',args,uniforms:new Map(this.uniforms),bindings:new Map(this.bindings)});}
}
const originalFetch=globalThis.fetch,originalBitmap=globalThis.createImageBitmap;
globalThis.fetch=async url=>{
 const path=fileURLToPath(url),bytes=await fs.readFile(path);
 return {ok:true,status:200,json:async()=>JSON.parse(bytes),blob:async()=>({bytes,url:path})};
};
globalThis.createImageBitmap=async blob=>({width:blob.bytes.readUInt32BE(16),height:blob.bytes.readUInt32BE(20),url:blob.url,close(){}});
try{
 const gl=new GL(),sentinel=gl.bindings.get(1),teams=Array.from({length:8},()=>[1,1,1]);
 const disabled=await loadOriginalTerrain(gl,false,teams);check(!disabled.enabled&&gl.textures.length===0,'Disabled ruleset does not load original images');
 const module=await loadOriginalTerrain(gl,true,teams);check(module.enabled,'Complete native terrain loader');check(module.errors.length===0,'No loader errors');
 const h=new Float32Array(257**2).fill(.3),types=new Uint8Array(256**2).fill(3),vp=new Float32Array(16);
 check(module.updateMap({heights:h,types}),'Initial field upload');
 check(!module.updateMap({heights:new Float32Array(h),types:new Uint8Array(types)}),'Resource-only map dirty does not recompute unchanged fields');
 module.drawTerrain({vao:{test:'terrain'},vp});module.drawWater({vao:{test:'water'},vp,time:10,eye:[1,2,3]});
 check(gl.draws.length===2,'Both passes draw');check(gl.draws[0].args[1]===256*256*6,'Full mesh index count');
 check(gl.bindings.get(1)===sentinel,'Legacy minimap texture binding is preserved');check(gl.unit===0,'Active texture restored');
 check(gl.draws[0].uniforms.get('uMask')===11,'Native alpha mask bound');check(gl.draws[0].uniforms.get('uWeights')===12,'Categorical weights bound');
 check(gl.draws[1].bindings.get(0).width===512,'Original Waterbump resolution retained');
 check(gl.draws[1].uniforms.get('uHasReflection')===0,'Sky reflection adapter explicit');
 module.drawWater({vao:{},vp,reflectionTexture:{fixture:'reflection'},reflectionVP:vp});
 check(gl.draws[2].uniforms.get('uHasReflection')===1,'Optional native distortion reflection path');
 await module.setSeason(0);check(module.metrics.seasonSlot===0,'Explicit winter slot loads');
 const arrays=gl.textures.filter(t=>t.depth===12);check(arrays.length===8,'Eight role arrays contain all twelve months');
 for(const t of arrays)check(t.images.length===12&&t.width===256&&t.height===256,'Complete original monthly images');
 module.setSeasonPhase({month0:1,month1:2,blend:.375});module.drawTerrain({vao:{},vp});
 check(gl.draws.at(-1).uniforms.get('uSeasonBlend')===.375,'Continuous monthly mix reaches the draw');
 check(gl.draws.at(-1).uniforms.get('uSeasonMonths').join(',')==='1,2','Adjacent native monthly layers reach the draw');
 for(const phase of [{month0:-1},{month0:12},{month0:0,month1:5},{month0:0,blend:NaN},{month0:0,blend:1.01}]){assert.throws(()=>module.setSeasonPhase(phase),RangeError);checks++;}
 for(const bad of [-1,12,.5,NaN]){await assert.rejects(module.setSeason(bad),RangeError);checks++;}
 const terrainCalls=module.metrics.terrainCalls;
 module.drawTerrain({vao:{},vp,clipPlane:[0,1,0,0],capture:true});check(gl.draws.at(-1).uniforms.get('uClipEnabled')===1,'Reflection terrain enables world clipping');
 check(gl.draws.at(-1).uniforms.get('uClipPlane').join(',')==='0,1,0,0','Reflection clipping uses original world y plane');check(module.metrics.terrainCalls===terrainCalls,'Reflection capture preserves main-pass counters');
 module.drawTerrain({vao:{},vp});check(gl.draws.at(-1).uniforms.get('uClipEnabled')===0,'Main pass resets reflection clipping');
 for(const clipPlane of [[0,1,0], [0,1,0,NaN], 'abcd']){assert.throws(()=>module.drawTerrain({vao:{},vp,clipPlane}),RangeError);checks++;}
 const report={format:'openepoch-native-terrain-contract-tests-v1',checks,passed:true,
  imageCount:module.metrics.images,initialSeasonSlot:6,explicitWinterSlotTested:true,
  noBrowserOrUI:true,scope:'Real source manifest/images through loader with stateful GL contract fake; separate ANGLE GPU report covers actual shaders/pixels.'};
 const root=new URL('../../EE2-Research/analysis/',import.meta.url);
 await fs.writeFile(new URL('native-terrain-contract-tests.json',root),JSON.stringify(report,null,2)+'\n');
 await fs.writeFile(new URL('native-terrain-shaders.json',root),JSON.stringify({programs:[{name:'native-terrain',...TERRAIN_SHADERS},{name:'native-water',...WATER_SHADERS}]},null,2)+'\n');
 console.log(JSON.stringify(report,null,2));
}finally{globalThis.fetch=originalFetch;globalThis.createImageBitmap=originalBitmap;}
