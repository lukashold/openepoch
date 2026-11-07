import {saveSceneGLState,restoreSceneGLState} from './original-reflection.js';

const f32=Math.fround, tilt=f32(.45), height=f32(.06), byte=f32(1/255);
const tau=f32(2*Math.PI), halfPi=f32(Math.PI/2);
const localQuad=[[-1,1,height],[1,1,height],[-1,-1,height],[1,-1,height]];
const uv=[[0,0],[0,1],[1,0],[1,1]];
const indices=new Uint16Array([0,3,1,0,2,3]);
const identity=[1,0,0,0,1,0,0,0,1];
const assets={
 good:['local-original/priest-fx/glow_good.png','4d34314b3df9e2e32376412bcdd3d964be7c0b71ec6245a34333e887e6e211ef'],
 bad:['local-original/capture-glow/glow_bad.png','0c44c9df52c3c64c866f22bfc732e6aa210c782e2742b17dc02d9ab8f9962d36'],
 mixed:['local-original/spy-advanced-commands/glow_both.png','0768dfeb5436fd368d57ce6732ff6af6bc173a0d5c7c74354b9ad51eb8667615'],
};

function product(a,b){
 const out=new Array(9);
 for(let row=0;row<9;row+=3){
  for(let column=0;column<3;column++)out[row+column]=f32(a[row]*b[column]+a[row+1]*b[3+column]+a[row+2]*b[6+column]);
 }
 return out;
}

function inverse([a,b,c,d,e,f,g,h,i]){
 const cof=[e*i-f*h,c*h-b*i,b*f-c*e,f*g-d*i,a*i-c*g,c*d-a*f,d*h-e*g,b*g-a*h,a*e-b*d].map(f32);
 const determinant=a*cof[0]+b*cof[3]+c*cof[6];
 if(Math.abs(determinant)<=f32(.000001))return null;
 return cof.map(value=>f32(value/determinant));
}

function axisRotation(x,y,z){
 const [cx,cy,cz]=[x,y,z].map(value=>f32(Math.cos(value)));
 const [sx,sy,sz]=[x,y,z].map(value=>f32(Math.sin(value)));
 return product([1,0,0,0,cx,sx,0,-sx,cx],product([cy,0,-sy,0,1,0,sy,0,cy],[cz,sz,0,-sz,cz,0,0,0,1]));
}

// Z-up row-major basis from VisualGlow world rotation. 00627ED0 writes
// inverse(parent)*reducedRotation into the attached geometry's local basis.
export function airGlowBasis(basis){
 if(basis?.length!==9||!Array.from(basis).every(Number.isFinite))return null;
 const matrix=Array.from(basis,f32),inv=inverse(matrix);
 if(!inv)return null;
 const y=-Math.asin(Math.max(-1,Math.min(1,matrix[2])));
 let x,z;
 if(y>=Math.PI/2||y<=-Math.PI/2){
  x=(y>0?-1:1)*Math.atan2(matrix[3],matrix[4]);z=0;
 }else{
  x=-Math.atan2(-matrix[5],matrix[8]);z=-Math.atan2(-matrix[1],matrix[0]);
 }
 const reduced=axisRotation(f32(f32(x)*tilt),f32(f32(Math.max(-halfPi,Math.min(halfPi,y)))*tilt),f32(z));
 const local=product(inv,reduced);
 return {local,world:product(matrix,local)};
}

export function airGlowPulse(phase){
 if(!Number.isFinite(phase))return null;
 const ratio=(f32(Math.sin(f32(phase%tau)))+1)*.5;
 return {scale:f32(10*f32(f32(.95)+(f32(1.15)-f32(.95))*ratio)),alpha:Math.trunc(100+50*ratio)};
}

export function airGlowQuad({origin,basis,yaw=0,parentLocalScale=1,parentWorldScale=parentLocalScale},pulse){
 if(origin?.length!==3||!Array.from(origin).every(Number.isFinite)||!Number.isFinite(yaw)||
    !(parentLocalScale>0)||!(parentWorldScale>0)||!Number.isFinite(parentLocalScale+parentWorldScale)||!pulse)return null;
 const rotation=basis===undefined?(yaw===0?identity:axisRotation(0,0,f32(yaw))):airGlowBasis(basis)?.world;
 if(!rotation)return null;
 const worldScale=f32(f32(parentWorldScale*f32(1/f32(parentLocalScale)))*pulse.scale);
 // Model imports use source X/Z/-Y. Browser yaw is already in that basis.
 const dot=(row,[x,y,z])=>rotation[row]*x+rotation[row+1]*y+rotation[row+2]*z;
 return new Float32Array(localQuad.flatMap((point,i)=>[
  f32(origin[0]+worldScale*dot(0,point)),f32(origin[1]+worldScale*dot(6,point)),f32(origin[2]-worldScale*dot(3,point)),...uv[i],
 ]));
}

const vs=`#version 300 es
layout(location=0) in vec3 aWorld;
layout(location=1) in vec2 aUV;
uniform mat4 uVP;
out vec3 vWorld;
out vec2 vUV;
void main(){vWorld=aWorld;vUV=aUV;gl_Position=uVP*vec4(aWorld,1.);}`;
const fs=`#version 300 es
precision highp float;
uniform sampler2D uTexture;
uniform float uPulseAlpha;
uniform bool uClip;
uniform vec4 uClipPlane;
in vec3 vWorld;
in vec2 vUV;
out vec4 outColor;
void main(){
 if(uClip&&dot(vec4(vWorld,1.),uClipPlane)<0.)discard;
 vec4 sampleColor=texture(uTexture,vUV);
 float alpha=sampleColor.a*uPulseAlpha;
 if(alpha<25./255.)discard;
 outColor=vec4(sampleColor.rgb,alpha);
}`;

function shaderProgram(gl){
 const program=gl.createProgram();
 for(const [kind,source] of [[gl.VERTEX_SHADER,vs],[gl.FRAGMENT_SHADER,fs]]){
  const shader=gl.createShader(kind);
  gl.shaderSource(shader,source);gl.compileShader(shader);
  if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS))throw Error(gl.getShaderInfoLog(shader));
  gl.attachShader(program,shader);gl.deleteShader(shader);
 }
 gl.linkProgram(program);
 if(!gl.getProgramParameter(program,gl.LINK_STATUS))throw Error(gl.getProgramInfoLog(program));
 return program;
}

export function createAirCrownGlowRenderer(gl,textures){
 const restore=saveSceneGLState(gl);
 let vao,vertices,index,program,uniforms;
 try{
  program=shaderProgram(gl);
  uniforms=Object.fromEntries(['uVP','uTexture','uPulseAlpha','uClip','uClipPlane'].map(name=>[name,gl.getUniformLocation(program,name)]));
  vao=gl.createVertexArray();vertices=gl.createBuffer();index=gl.createBuffer();
  gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,vertices);
  gl.enableVertexAttribArray(0);gl.vertexAttribPointer(0,3,gl.FLOAT,false,20,0);
  gl.enableVertexAttribArray(1);gl.vertexAttribPointer(1,2,gl.FLOAT,false,20,12);
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,index);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,indices,gl.STATIC_DRAW);
 }finally{restoreSceneGLState(gl,restore);}
 function draw({vp,rows,phase,clipPlane}){
  const pulse=airGlowPulse(phase);
  if(!pulse||!rows.length)return 0;
  const state=saveSceneGLState(gl);
  let count=0;
  try{
   gl.useProgram(program);gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,vertices);
   gl.uniformMatrix4fv(uniforms.uVP,false,vp);gl.uniform1i(uniforms.uTexture,0);
   gl.uniform1f(uniforms.uPulseAlpha,f32(pulse.alpha*byte));
   gl.uniform1i(uniforms.uClip,clipPlane?1:0);gl.uniform4fv(uniforms.uClipPlane,clipPlane??[0,0,0,0]);
   gl.activeTexture(gl.TEXTURE0);gl.enable(gl.DEPTH_TEST);gl.depthFunc(gl.LEQUAL);gl.depthMask(false);
   gl.disable(gl.CULL_FACE);gl.enable(gl.BLEND);gl.blendEquation(gl.FUNC_ADD);gl.blendFunc(gl.SRC_ALPHA,gl.ONE);
   for(const row of rows){
    const texture=textures.get(row.kind);
    if(!texture)continue;
    const data=airGlowQuad(row,pulse);
    if(!data)continue;
    gl.bindTexture(gl.TEXTURE_2D,texture);gl.bufferData(gl.ARRAY_BUFFER,data,gl.STREAM_DRAW);
    gl.drawElements(gl.TRIANGLES,indices.length,gl.UNSIGNED_SHORT,0);count++;
   }
  }finally{restoreSceneGLState(gl,state);}
  return count;
 }
 return Object.freeze({draw});
}

export async function loadAirCrownGlow(gl,{enabled,base=new URL('./',import.meta.url),fetchFile=globalThis.fetch,decode=globalThis.createImageBitmap}={}){
 if(!enabled)return null;
 const textures=new Map();
 for(const [kind,[path,expected]] of Object.entries(assets)){
  try{
   const url=new URL(path,base);url.searchParams.set('v',expected.slice(0,16));
   const response=await fetchFile(url);if(!response.ok)continue;
   const bytes=await response.arrayBuffer(),digest=new Uint8Array(await crypto.subtle.digest('SHA-256',bytes));
   if(Array.from(digest,value=>value.toString(16).padStart(2,'0')).join('')!==expected)continue;
   const bitmap=await decode(new Blob([bytes],{type:'image/png'}),{premultiplyAlpha:'none',colorSpaceConversion:'none'});
   const active=gl.getParameter(gl.ACTIVE_TEXTURE),flip=gl.getParameter(gl.UNPACK_FLIP_Y_WEBGL);
   gl.activeTexture(gl.TEXTURE0);
   const old=gl.getParameter(gl.TEXTURE_BINDING_2D),texture=gl.createTexture();
   try{
    if(bitmap.width!==128||bitmap.height!==128){gl.deleteTexture(texture);continue;}
    gl.bindTexture(gl.TEXTURE_2D,texture);gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,false);
    gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA,gl.RGBA,gl.UNSIGNED_BYTE,bitmap);
    for(const parameter of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D,parameter,gl.NEAREST);
    for(const parameter of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(gl.TEXTURE_2D,parameter,gl.REPEAT);
    textures.set(kind,texture);
   }catch(error){gl.deleteTexture(texture);throw error;}
   finally{bitmap.close();gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL,flip);gl.bindTexture(gl.TEXTURE_2D,old);gl.activeTexture(active);}
  }catch{/* Older private bundles may lack one color. */}
 }
 if(!textures.size)return null;
 try{return createAirCrownGlowRenderer(gl,textures);}
 catch{for(const texture of textures.values())gl.deleteTexture(texture);return null;}
}
