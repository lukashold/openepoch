// Optional original caster FX. Callers provide visible attachment instances in
// world space from their read-only event and render snapshots.
const vertex=`#version 300 es
layout(location=0) in vec3 aPosition;
layout(location=1) in vec2 aUV;
uniform mat4 uVP;
invariant gl_Position;
out vec2 vUV;
void main(){vUV=aUV;gl_Position=uVP*vec4(aPosition,1.);}`;
const fragment=`#version 300 es
precision highp float;
uniform sampler2D uBase;
uniform sampler2D uOrb;
uniform bool uUsesOrb;
uniform float uAlphaCut;
uniform vec3 uEmission;
uniform vec3 uSpecular;
in vec2 vUV;
out vec4 frag;
void main(){
 vec4 base=texture(uBase,vUV);
 if(base.a<=uAlphaCut)discard;
 vec3 rgb=base.rgb;
 if(uUsesOrb){
  vec3 orb=texture(uOrb,vUV).rgb;
  rgb=clamp(base.rgb*(vec3(1.)+uEmission)*orb+orb*uSpecular,0.,1.);
 }
 frag=vec4(rgb,base.a);
}`;
// The fixed-function dark/glow stages above use the exact original textures
// and animated material inputs. Their composite is a WebGL adapter.
export const PRIEST_EFFECT_SHADERS=Object.freeze({vertex,fragment});

const dot=(a,b)=>a.reduce((sum,value,i)=>sum+value*b[i],0);
const cross=(a,b)=>[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
const normal=v=>{const d=Math.hypot(...v);return d>1e-8?v.map(x=>x/d):null;};
const add=(a,b)=>a.map((x,i)=>x+b[i]);
const mul=(a,s)=>a.map(x=>x*s);
const validVec=v=>v?.length===3&&Array.from(v).every(Number.isFinite);
const v3=value=>typeof value==='number'?[value]:[value.x,value.y,value.z];

export function samplePriestKeyGroup(group,elapsed){
 const keys=group?.Keys;
 if(!keys?.length)return null;
 const after=keys.findIndex(key=>key.Time>=elapsed);
 if(after<0)return v3(keys.at(-1).Value);
 const next=keys[after];
 if(!after||next.Time===elapsed)return v3(next.Value);
 const previous=keys[after-1],t=(elapsed-previous.Time)/(next.Time-previous.Time);
 const left=v3(previous.Value),right=v3(next.Value);
 if(group.Interpolation!==2)return left.map((x,i)=>x+(right[i]-x)*t);
 const squared=t*t,cubed=squared*t;
 const forward=v3(previous.Forward),backward=v3(next.Backward);
 return left.map((x,i)=>(2*cubed-3*squared+1)*x+(cubed-2*squared+t)*forward[i]
  +(-2*cubed+3*squared)*right[i]+(cubed-squared)*backward[i]);
}

function localPoint(node,p){
 const out=[0,3,6].map(offset=>dot(p,node.rotation.slice(offset,offset+3)));
 return add(node.translation,mul(out,node.scale));
}

function actorOffset(point,yaw,scale){
 const [x,y,z]=point,c=Math.cos(yaw),s=Math.sin(yaw);
 return mul([x*c+y*s,z,x*s-y*c],scale);
}

export function projectPriestMesh(mesh,event,view){
 if(!mesh||event?.visible!==true||!validVec(event.position)||!Number.isFinite(event.yaw)
    ||!(event.scale>0)||!Number.isFinite(event.scale)||!(event.elapsed>=0&&event.elapsed<2))return null;
 const origin=add(event.position,actorOffset(mesh.parent.translation,event.yaw,event.scale));
 let transform;
 if(mesh.billboardMode===0){
  if(!validVec(view.forward)||!validVec(view.right))return null;
  const facing=normal(mul(view.forward,-1));
  const up=facing&&normal(cross(facing,view.right));
  const right=up&&normal(cross(up,facing));
  if(!right)return null;
  transform=p=>mul(add(add(mul(right,p[0]),mul(up,p[1])),mul(facing,p[2])),event.scale*mesh.parent.scale);
 }else{
  const keys=mesh.animation?.keys;
  const z=samplePriestKeyGroup(keys?.['XYZ Rotations']?.[2],event.elapsed)?.[0]??0;
  const scale=samplePriestKeyGroup(keys?.Scales,event.elapsed)?.[0]??mesh.parent.scale;
  const c=Math.cos(z),s=Math.sin(z);
  transform=p=>actorOffset([p[0]*c-p[1]*s,p[0]*s+p[1]*c,p[2]],event.yaw,event.scale*scale);
 }
 const positions=mesh.positions.map(p=>add(origin,transform(localPoint(mesh.shape,p))));
 const animatedColor=(id,fallback)=>{
  const ctrl=mesh.materialControllers.find(row=>row['Target Color']===id);
  return ctrl?samplePriestKeyGroup(ctrl.keys,event.elapsed):fallback;
 };
 return {positions,emission:animatedColor(3,[0,0,0]),specular:animatedColor(2,[0,0,0])};
}

function indices(mesh){
 const result=mesh.triangles.flat();
 for(const strip of mesh.strips){
  for(let i=2;i<strip.length;i++){
   const a=strip[i-2],b=strip[i-1],c=strip[i];
   if(a!==b&&b!==c&&c!==a)result.push(...(i%2?[b,a,c]:[a,b,c]));
  }
 }
 return new Uint16Array(result);
}

const textureUnits=[0,1];
function saveState(gl){
 const parameters=[gl.ACTIVE_TEXTURE,gl.CURRENT_PROGRAM,gl.VERTEX_ARRAY_BINDING,gl.ARRAY_BUFFER_BINDING,
  gl.DEPTH_FUNC,gl.DEPTH_WRITEMASK,gl.BLEND_SRC_RGB,gl.BLEND_DST_RGB,gl.BLEND_SRC_ALPHA,gl.BLEND_DST_ALPHA,
  gl.BLEND_EQUATION_RGB,gl.BLEND_EQUATION_ALPHA,gl.UNPACK_FLIP_Y_WEBGL];
 const [active,program,vao,buffer,depth,write,...rest]=parameters.map(key=>gl.getParameter(key));
 const caps=[gl.BLEND,gl.DEPTH_TEST,gl.CULL_FACE].map(key=>[key,gl.isEnabled(key)]);
 const textures=textureUnits.map(unit=>{gl.activeTexture(gl.TEXTURE0+unit);return gl.getParameter(gl.TEXTURE_BINDING_2D);});
 gl.activeTexture(active);
 return {active,program,vao,buffer,depth,write,rest,caps,textures};
}
function restoreState(gl,state){
 const [src,dst,srcAlpha,dstAlpha,rgb,alpha,flip]=state.rest;
 textureUnits.forEach((unit,i)=>{gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(gl.TEXTURE_2D,state.textures[i]);});
 gl.activeTexture(state.active);gl.useProgram(state.program);gl.bindVertexArray(state.vao);gl.bindBuffer(gl.ARRAY_BUFFER,state.buffer);
 gl.depthFunc(state.depth);gl.depthMask(state.write);gl.blendFuncSeparate(src,dst,srcAlpha,dstAlpha);gl.blendEquationSeparate(rgb,alpha);
 gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,flip);
 for(const [key,value] of state.caps)value?gl.enable(key):gl.disable(key);
}

function program(gl){
 const result=gl.createProgram();
 try{
  for(const [type,source] of [[gl.VERTEX_SHADER,vertex],[gl.FRAGMENT_SHADER,fragment]]){
   const shader=gl.createShader(type);
   try{
    gl.shaderSource(shader,source);gl.compileShader(shader);
    if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(shader));
    gl.attachShader(result,shader);
   }finally{gl.deleteShader(shader);}
  }
  gl.linkProgram(result);
  if(!gl.getProgramParameter(result,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(result));
  return result;
 }catch(error){gl.deleteProgram(result);throw error;}
}

export async function loadPriestEffects(gl,{enabled=false,base=new URL('./',import.meta.url),
 fetchFile=globalThis.fetch,decode=globalThis.createImageBitmap}={}){
 if(!enabled)return null;
 const release=[];
 let manifest,images;
 try{
  const response=await fetchFile(new URL('local-original/priest-fx/manifest.json',base));
  if(!response.ok)return null;
  manifest=await response.json();
  if(manifest.format!=='ee2-native-priest-fx-v1')return null;
  images=new Map();
  for(const key of ['halo','orb01','water_splash03']){
   const record=manifest.textures[key];
   if(record.url!==`local-original/priest-fx/${key}.png`)throw Error(`Invalid Priest FX ${key} path`);
   const file=await fetchFile(new URL(record.url,base));
   if(!file.ok)throw Error(`Missing Priest FX ${key}`);
   const image=await decode(await file.blob(),{premultiplyAlpha:'none',colorSpaceConversion:'none'});
   images.set(key,image);
   if(image.width!==record.size[0]||image.height!==record.size[1])throw Error(`Invalid Priest FX ${key} size`);
  }
  // Only take WebGL state after asynchronous fetches and decodes finish.
  const state=saveState(gl);
  try{
   const shader=program(gl);release.push(()=>gl.deleteProgram(shader));
   const uniforms=Object.fromEntries(['uVP','uBase','uOrb','uUsesOrb','uAlphaCut','uEmission','uSpecular']
    .map(name=>[name,gl.getUniformLocation(shader,name)]));
   const textures=new Map();
   for(const [key,image] of images){
    const texture=gl.createTexture();release.push(()=>gl.deleteTexture(texture));
    gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,texture);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);
    for(const flag of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D,flag,gl.LINEAR);
    for(const flag of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(gl.TEXTURE_2D,flag,gl.REPEAT);
    textures.set(key,texture);
   }
   const geometries={};
   for(const kind of ['bless','convert']){
    const mesh=manifest.effects[kind].mesh;
    const index=indices(mesh);
    if(!index.length||index.some(i=>i>=mesh.positions.length)||mesh.depth.Flags!==1||mesh.depth.Function!==3
       ||(mesh.alpha.Flags&511)!==13||!textures.has(mesh.textures.base.texture))throw Error(`Invalid Priest FX ${kind} mesh`);
    const vao=gl.createVertexArray(),vertices=gl.createBuffer(),element=gl.createBuffer();
    release.push(()=>{gl.deleteVertexArray(vao);gl.deleteBuffer(vertices);gl.deleteBuffer(element);});
    gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,vertices);
    for(const [loc,length,offset] of [[0,3,0],[1,2,12]]){
     gl.enableVertexAttribArray(loc);gl.vertexAttribPointer(loc,length,gl.FLOAT,false,20,offset);
    }
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,element);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,index,gl.STATIC_DRAW);
    const data=new Float32Array(mesh.positions.length*5);
    mesh.uvs.forEach((uv,i)=>data.set(uv,i*5+3));
    geometries[kind]={vao,vertices,mesh,data,count:index.length};
   }
   let disposed=false;
   return {
    dispose(){if(!disposed){disposed=true;release.reverse().forEach(remove=>remove());}},
    draw({vp,right,f:forward=[vp[3],vp[7],vp[11]],effects=[]}){
     if(disposed||!effects.length)return 0;
     const drawState=saveState(gl);
     let count=0;
     try{
      gl.useProgram(shader);gl.uniformMatrix4fv(uniforms.uVP,false,vp);
      gl.uniform1i(uniforms.uBase,0);gl.uniform1i(uniforms.uOrb,1);
      gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);gl.depthMask(false);gl.disable(gl.CULL_FACE);
      gl.enable(gl.BLEND);gl.blendEquation(gl.FUNC_ADD);gl.blendFunc(gl.SRC_ALPHA,gl.ONE);
      gl.activeTexture(gl.TEXTURE1);gl.bindTexture(gl.TEXTURE_2D,textures.get('orb01'));
      gl.activeTexture(gl.TEXTURE0);
      for(const event of effects){
       if(event?.kind!=='bless'&&event?.kind!=='convert')continue;
       const {mesh,vao,vertices,data,count:indexCount}=geometries[event.kind];
       const pose=projectPriestMesh(mesh,event,{forward,right});
       if(!pose)continue;
       pose.positions.forEach((p,i)=>data.set(p,i*5));
       gl.bindTexture(gl.TEXTURE_2D,textures.get(mesh.textures.base.texture));
       gl.uniform1i(uniforms.uUsesOrb,mesh.textures.dark?1:0);
       gl.uniform1f(uniforms.uAlphaCut,mesh.alpha.Flags&512?mesh.alpha.Threshold/255:-1);
       gl.uniform3fv(uniforms.uEmission,pose.emission);gl.uniform3fv(uniforms.uSpecular,pose.specular);
       gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,vertices);gl.bufferData(gl.ARRAY_BUFFER,data,gl.STREAM_DRAW);
       gl.drawElements(gl.TRIANGLES,indexCount,gl.UNSIGNED_SHORT,0);count++;
      }
      return count;
     }finally{restoreState(gl,drawState);}
    },
   };
  }finally{restoreState(gl,state);}
 }catch{
  release.reverse().forEach(remove=>remove());
  return null;
 }finally{for(const image of images?.values()??[])image.close();}
}
