import {saveSceneGLState,restoreSceneGLState} from './original-reflection.js';

const f=Math.fround;
const reciprocal=f(1/32767),radians=0.017453333333333335;
const splashColors=[[0,0,0,255],[255,109,1,255],[255,31,1,255]];
const materials=[
 {file:'fx_land_explosion1.png',width:512,height:256,columns:8,rows:4,seed:2,wind:f(.4),
  hash:'577fa557be59f5f7b46b678459478ac62918573deeeb8ad1fa11fcf1e6e07b43',
  particles:[0,.2,.3].map((born,i)=>({born:f(born),life:2,from:i===0?1:.5,size:1,grow:f(.02),speed:f(.2),
   colors:[splashColors[i],splashColors[i],[0,0,0,128],[0,0,0,0]],times:[0,f(.10),f(i===0?1:.75),2]}))},
 {file:'flash.png',width:256,height:256,columns:1,rows:1,seed:0,wind:0,
  hash:'a877ecf6855a94b4262a11fee131760dd2ad6744140f8678991eafd46ea70389',
  particles:[1.5,2.5].map((life,i)=>({born:0,life,from:1.5,size:1.5,grow:f(.05),speed:0,
   colors:[[0,0,0,i===0?255:100],[0,0,0,i===0?255:100],[0,0,0,i===0?128:50],[0,0,0,0]],
   times:[0,f(.15),f(.55),life]}))},
];

const VS=`#version 300 es
layout(location=0) in vec3 position;
layout(location=1) in vec2 uv;
layout(location=2) in vec4 color;
layout(location=3) in float elapsed;
uniform mat4 vp;
out vec2 texCoord;
out vec4 diffuse;
flat out float sourceTime;
void main(){
 gl_Position=vp*vec4(position,1.0);
 texCoord=uv;
 diffuse=color;
 sourceTime=elapsed;
}`;
const FS=`#version 300 es
precision highp float;
uniform sampler2D atlas;
in vec2 texCoord;
in vec4 diffuse;
flat in float sourceTime;
out vec4 frag;
void main(){
 vec4 texel=texture(atlas,texCoord);
 gl_FragDepth=clamp(gl_FragCoord.z+texel.r*(-0.5)*sourceTime*gl_FragCoord.w,0.0,1.0);
 frag=clamp(vec4(texel.rgb+diffuse.rgb,texel.a*diffuse.a),0.0,1.0);
}`;

const finite=(value,n)=>value?.length===n&&Array.prototype.every.call(value,Number.isFinite);
const corners=[[0,0],[1,0],[1,1],[0,0],[1,1],[0,1]];
const componentScale=f(1/255);

// 006242B0 tests the original descriptor's radius-1 sphere against six
// inward-facing camera planes. VP encodes the same portable clip volume.
export function guidedImpactInside(vp,x,y,z,strict=false){
 if(!finite(vp,16)||![x,y,z].every(Number.isFinite))return false;
 for(let axis=0;axis<3;axis++)for(const sign of [1,-1]){
  const nx=vp[3]+sign*vp[axis],ny=vp[7]+sign*vp[4+axis],nz=vp[11]+sign*vp[8+axis];
  const offset=vp[15]+sign*vp[12+axis];
  const distance=nx*x+ny*z+nz*y+offset, boundary=-Math.hypot(nx,ny,nz);
  if(distance<boundary||(strict&&distance===boundary))return false;
 }
 return true;
}

function launchVelocity(layer,speed){
 // 00617090–00617242 takes three words from a private 1664525/1013904223 LCG.
 const random=()=>((layer.seed=(Math.imul(layer.seed,1664525)+1013904223)>>>0)>>>16)&0x7fff;
 const rotation=()=>f(180*radians*random()*reciprocal-90*radians);
 const a=rotation(),b=rotation(),velocity=speed*random()*reciprocal;
 const cosX=f(Math.cos(a)),sinX=f(Math.sin(a)),cosY=f(Math.cos(b)),sinY=f(Math.sin(b));
 // 008E6480/008E64C0 build X and Y. 008E69D0 uses the third row of X*Y
 // for the Z basis. The browser maps native XYZ to X,Z,-Y.
 return [f(f(sinY*cosX)*velocity),f(f(cosY*cosX)*velocity),f(sinX*velocity)];
}

function packColor(def,age){
 const {colors,times}=def;
 let i=0;
 while(i<3&&age>=times[i+1])i++;
 const t=i===3?0:(age-times[i])/(times[i+1]-times[i]);
 return colors[i].map((byte,c)=>{
  const start=f(byte*componentScale),end=f((colors[i+1]?.[c]??byte)*componentScale);
  const unpacked=f(f(start+(end-start)*t)*255),base=Math.floor(unpacked),fraction=unpacked-base;
  return (base+Number(fraction>.5||fraction===.5&&base%2===1))/255;
 });
}

function makeParticle(def,layer,order,origin){
 const velocity=launchVelocity(layer,def.speed),speed=Math.hypot(...velocity);
 // Both active descriptors omit UseDefinedTranslate. The VTiny path offsets
 // along nonzero velocity by one eighth of EndSize (006172AC–00617386).
 const distance=speed>0?f(def.size*.125)/speed:0;
 return {def,velocity,remaining:def.life,age:0,frame:0,flip:order%4,
  position:origin.map((value,i)=>f(value+velocity[i]*distance))};
}

export function createGuidedImpactFrames({flashSeed=()=>Math.floor(Date.now()/1000)}={}){
 const active=[],pending=[];
 let last=0;
 const reset=seconds=>{active.length=0;pending.length=0;last=Number.isFinite(seconds)?seconds:0;};
 return {
  reset,
  spawn(x,y,z){
   if(![x,y,z].every(Number.isFinite))return;
   if(active.length+pending.length===256)(active.length?active:pending).shift();
   pending.push({origin:[x,z,y],layers:materials.map(row=>({seed:row.seed||flashSeed(),age:0,next:0,particles:[]}))});
  },
  advance(seconds,wind=[0,0]){
   if(!Number.isFinite(seconds)||seconds<last){reset(seconds);return;}
   const delta=f(seconds-last);last=seconds;
   for(const effect of active)for(const [i,row] of materials.entries()){
    const layer=effect.layers[i];
    layer.age=f(layer.age+delta);
    while(row.particles[layer.next]?.born<=layer.age){
     layer.particles.push(makeParticle(row.particles[layer.next],layer,layer.next,effect.origin));layer.next++;
    }
    for(let j=0;j<layer.particles.length;j++){
     const particle=layer.particles[j];
     particle.remaining=f(particle.remaining-delta);
     particle.age=f(particle.age+delta);
     if(particle.remaining<=0){
      const tail=layer.particles.pop();
      if(j<layer.particles.length)layer.particles[j]=tail;
      j--;continue;
     }
     if(!delta&&particle.center)continue;
     if(i===0){
      particle.frame=f(particle.frame+16*delta);
      if(particle.frame>=32)particle.frame=0;
     }
     for(let k=0;k<3;k++)particle.position[k]=f(particle.position[k]+particle.velocity[k]*delta);
     particle.center=[f(particle.position[0]+wind[0]*row.wind*delta),particle.position[1],
      f(particle.position[2]-wind[1]*row.wind*delta)];
    }
   }
   for(let i=active.length-1;i>=0;i--){
    if(active[i].layers.every((layer,j)=>layer.next===materials[j].particles.length&&!layer.particles.length))active.splice(i,1);
   }
   // The source composite already drained its child queue when the explosion
   // wrapper attached these two effects. They update on the next context pass.
   active.push(...pending);pending.length=0;
  },
  sample(visible){
   const layers=[[],[]];
   for(const {origin,layers:effectLayers} of active){
    if(!visible(origin[0],origin[2]))continue;
    for(const [i,row] of materials.entries()){
     const layer=effectLayers[i];
     for(const {def,age,frame,flip,center} of layer.particles){
      const bitmap=Math.trunc(frame),column=bitmap%row.columns,line=Math.trunc(bitmap/row.columns);
      const size=age>=def.grow?def.size:f(def.from+(def.size-def.from)*age/def.grow);
      layers[i].push({center,color:packColor(def,age),size,time:layer.age,uv:[column/row.columns,line/row.rows],
       step:[1/row.columns,1/row.rows],flip:i===0?flip:0});
     }
    }
   }
   return layers;
  },
 };
}

function makeProgram(gl){
 const program=gl.createProgram();
 try{
  for(const [type,source] of [[gl.VERTEX_SHADER,VS],[gl.FRAGMENT_SHADER,FS]]){
   const shader=gl.createShader(type);
   try{
    gl.shaderSource(shader,source);gl.compileShader(shader);
    if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(shader));
    gl.attachShader(program,shader);
   }finally{gl.deleteShader(shader);}
  }
  gl.linkProgram(program);
  if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(program));
  return program;
 }catch(error){gl.deleteProgram(program);throw error;}
}

export async function loadGuidedImpactEffects(gl,enabled){
 if(!enabled)return null;
 const release=[],images=[];
 try{
  for(const {file,width,height,hash} of materials){
   const response=await fetch(new URL(`local-original/guided-herc-impact/${file}`,import.meta.url));
   if(!response.ok)return null;
   const bytes=await response.arrayBuffer();
   const signature=Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),n=>n.toString(16).padStart(2,'0')).join('');
   if(signature!==hash)return null;
   const bitmap=await createImageBitmap(new Blob([bytes],{type:'image/png'}),{premultiplyAlpha:'none',colorSpaceConversion:'none'});
   images.push(bitmap);
   if(bitmap.width!==width||bitmap.height!==height)return null;
  }
  const before=saveSceneGLState(gl),flip=gl.getParameter(gl.UNPACK_FLIP_Y_WEBGL);
  try{
   const program=makeProgram(gl);release.push(()=>gl.deleteProgram(program));
   const vp=gl.getUniformLocation(program,'vp'),atlas=gl.getUniformLocation(program,'atlas');
   gl.activeTexture(gl.TEXTURE0);gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);
   const textures=images.map(image=>{
    const texture=gl.createTexture();release.push(()=>gl.deleteTexture(texture));
    gl.bindTexture(gl.TEXTURE_2D,texture);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,image);
    for(const key of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D,key,gl.LINEAR);
    for(const key of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(gl.TEXTURE_2D,key,gl.CLAMP_TO_EDGE);
    return texture;
   });
   const vao=gl.createVertexArray(),buffer=gl.createBuffer();
   release.push(()=>{gl.deleteVertexArray(vao);gl.deleteBuffer(buffer);});
   gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
   for(const [index,length,offset] of [[0,3,0],[1,2,12],[2,4,20],[3,1,36]]){
    gl.enableVertexAttribArray(index);gl.vertexAttribPointer(index,length,gl.FLOAT,false,40,offset);
   }
   const frames=createGuidedImpactFrames();
   return Object.assign(frames,{draw(view,visible){
    if(!finite(view.right,3)||!finite(view.u,3))return [0,0];
    const layers=frames.sample(visible),counts=layers.map(layer=>layer.length);
    if(!counts[0]&&!counts[1])return counts;
    const state=saveSceneGLState(gl);
    try{
     gl.useProgram(program);gl.uniformMatrix4fv(vp,false,view.vp);gl.uniform1i(atlas,0);
     gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);gl.depthMask(false);gl.disable(gl.CULL_FACE);
     gl.enable(gl.BLEND);gl.blendEquation(gl.FUNC_ADD);
     gl.activeTexture(gl.TEXTURE0);gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
     for(const [i,particles] of layers.entries()){
      if(!particles.length)continue;
      const vertices=[];
      for(const {center,color,size,time,uv,step,flip} of particles)for(const [u,v] of corners){
       const du=(2*u-1)*size,dv=(1-2*v)*size;
       vertices.push(...center.map((value,k)=>value+du*view.right[k]+dv*view.u[k]),
        uv[0]+((flip&1)?1-u:u)*step[0],uv[1]+((flip&2)?1-v:v)*step[1],...color,time);
      }
      gl.bindTexture(gl.TEXTURE_2D,textures[i]);gl.blendFunc(gl.SRC_ALPHA,i===0?gl.ONE_MINUS_SRC_ALPHA:gl.ONE);
      gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(vertices),gl.STREAM_DRAW);
      gl.drawArrays(gl.TRIANGLES,0,vertices.length/10);
     }
     return counts;
    }finally{restoreSceneGLState(gl,state);}
   }});
  }finally{restoreSceneGLState(gl,before);gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,flip);}
 }catch(error){
  release.reverse().forEach(remove=>remove());
  console.warn('HERC impact effects:',error);
  return null;
 }finally{images.forEach(image=>image.close());}
}
