import {createLegacyEmitter,createLegacyRand} from './legacy-particles.js?v=native47';
import {saveSceneGLState,restoreSceneGLState} from './original-reflection.js';
import {life,middleEnd} from './guided-herc-beam-math.js?v=herc-beam1';

const f=Math.fround;
export const splashEnd=f(life+f(1.5));
const identity=[1,0,0,0,1,0,0,0,1];
const noKeys={Scales:{'Num Keys':0}};
const dot=(a,b)=>a.reduce((sum,value,i)=>sum+value*b[i],0);
const matrixPoint=(rotation,point)=>[0,1,2].map(i=>dot(rotation.slice(3*i,3*i+3),point));
const worldPoint=(anchor,point)=>anchor.point.map((value,i)=>value+anchor.scale*dot(anchor.axes.map(axis=>axis[i]),point));
const scene=([x,y,z])=>[x,z,y];
const corners=[[0,0],[0,1],[1,0],[1,0],[0,1],[1,1]];
const VS=`#version 300 es
layout(location=0) in vec3 aPosition;
layout(location=1) in vec2 aUV;
layout(location=2) in vec4 aColor;
uniform mat4 uVP;
out vec2 vUV;
out vec4 vColor;
void main(){vUV=aUV;vColor=aColor;gl_Position=uVP*vec4(aPosition,1.);}`;
const FS=`#version 300 es
precision highp float;
uniform sampler2D uBeamChild;
uniform float uCutoff;
in vec2 vUV;
in vec4 vColor;
out vec4 color;
void main(){
 vec4 value=texture(uBeamChild,vUV)*vColor;
 if(value.a<=uCutoff)discard;
 color=value;
}`;

function particleDescriptor(row){
 // The legacy controller operates in the particle shape's frame. These
 // authored emitters are static, and all three local scales are one.
 const a=row.parent,b=row.shape;
 const cols=[0,1,2].map(i=>matrixPoint(a.rotation,[b.rotation[i],b.rotation[3+i],b.rotation[6+i]]));
 const parent={scale:1,rotation:[0,1,2].flatMap(i=>cols.map(column=>column[i])),
  translation:matrixPoint(a.rotation,b.translation).map((value,i)=>value+a.translation[i])};
 return {...row,parent,shape:{scale:1,rotation:identity,translation:[0,0,0]},emitterKeys:noKeys};
}

export function createBeamChildTimeline(data,detail){
 const descriptors=Object.fromEntries(Object.entries(data).map(([kind,child])=>[kind,child.emitters.map(particleDescriptor)]));
 for(const rows of Object.values(descriptors))for(const row of rows)createLegacyEmitter(row,()=>0);
 function create(seed){return {rand:createLegacyRand(seed),muzzle:null,splash:null};}
 function update(state,age,reached){
  for(const [kind,enabled,deadline] of [['muzzle',detail>1,middleEnd],['splash',detail!==0&&reached,splashEnd]]){
   const child=state[kind];
   if(age>=deadline){state[kind]=false;continue;}
   if(child===false)continue;
   if(!child){
    if(enabled)state[kind]={start:age,elapsed:0,emitters:descriptors[kind].map(row=>createLegacyEmitter(row,state.rand))};
    continue;
   }
   const elapsed=f(age-child.start);
   if(elapsed<=child.elapsed)continue;
   for(const emitter of child.emitters)emitter.step(elapsed);
   child.elapsed=elapsed;
  }
 }
 return {create,update};
}

export function scalarKey(animation,elapsed){
 const {controller,keys}=animation,duration=controller['Stop Time'];
 const loop=((controller.Flags>>1)&3)===0;
 const time=loop?f(elapsed%duration):Math.min(elapsed,duration);
 const next=keys.Keys.findIndex(key=>key.Time>=time);
 if(next===0)return keys.Keys[0].Value;
 if(next<0)return keys.Keys.at(-1).Value;
 const a=keys.Keys[next-1],b=keys.Keys[next],t=(time-a.Time)/(b.Time-a.Time);
 if(keys.Interpolation===1)return f(a.Value+(b.Value-a.Value)*t);
 const t2=t*t,t3=t2*t;
 return f((2*t3-3*t2+1)*a.Value+(t3-2*t2+t)*a.Forward+(-2*t3+3*t2)*b.Value+(t3-t2)*b.Backward);
}

function packColor(value){
 const byte=f(value*255),lower=Math.floor(byte),fraction=byte-lower;
 return (lower+(fraction>.5||fraction===.5&&lower%2===1?1:0))/255;
}

function billboardVertices(mesh,elapsed,anchor,view){
 const {right,u:up,f:forward}=view;
 const scale=anchor.scale*scalarKey(mesh.scale,elapsed);
 const alpha=mesh.alpha?scalarKey(mesh.alpha,elapsed):1;
 if(alpha<=0||scale<=0)return [];
 const center=scene(worldPoint(anchor,mesh.parent.translation)),list=[];
 for(const i of mesh.indices){
  const [x,y,z]=matrixPoint(mesh.shape.rotation,mesh.positions[i]).map((value,j)=>mesh.shape.translation[j]+mesh.shape.scale*value);
  const position=center.map((value,j)=>value+scale*(x*right[j]+y*up[j]-z*forward[j]));
  list.push(...position,...mesh.uv[i],1,1,1,alpha);
 }
 return list;
}

function particleVertices(particles,anchor,{right,u:up}){
 const list=[];
 for(const row of particles){
  const center=scene(worldPoint(anchor,row.position)),radius=row.radius*anchor.scale,color=row.color.map(packColor);
  for(const [u,v] of corners){
   const x=(2*u-1)*radius,y=(1-2*v)*radius;
   list.push(...center.map((value,i)=>value+right[i]*x+up[i]*y),u,v,...color);
  }
 }
 return list;
}

export function createBeamChildLayer(gl,data,textures,program,detail){
 const timeline=createBeamChildTimeline(data,detail);
 const initial=saveSceneGLState(gl);
 let shader,vao,buffer;
 try{
  shader=program(gl,VS,FS);vao=gl.createVertexArray();buffer=gl.createBuffer();
  gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
  for(const [location,count,offset] of [[0,3,0],[1,2,12],[2,4,20]]){
   gl.enableVertexAttribArray(location);gl.vertexAttribPointer(location,count,gl.FLOAT,false,36,offset);
  }
 }finally{restoreSceneGLState(gl,initial);}
 const matrix=gl.getUniformLocation(shader,'uVP'),cutoff=gl.getUniformLocation(shader,'uCutoff');
 const sampler=gl.getUniformLocation(shader,'uBeamChild');
 function draw(view,effects){
  const result={muzzle:0,splash:0,particles:0};
  if(!effects.length)return result;
  const before=saveSceneGLState(gl);
  try{
   gl.enable(gl.BLEND);gl.blendEquation(gl.FUNC_ADD);gl.disable(gl.CULL_FACE);
   gl.useProgram(shader);gl.uniformMatrix4fv(matrix,false,view.vp);gl.uniform1i(sampler,0);
   gl.activeTexture(gl.TEXTURE0);gl.bindVertexArray(vao);gl.bindBuffer(gl.ARRAY_BUFFER,buffer);
   for(const {state,muzzle,point} of effects){
    for(const [kind,anchor] of [['muzzle',muzzle],['splash',{...muzzle,point}]]){
     const child=state[kind];
     if(!child)continue;
     const source=data[kind];
     const render=(material,vertices)=>{
      if(!vertices.length)return;
      if(material.depthFlags&1)gl.enable(gl.DEPTH_TEST);else gl.disable(gl.DEPTH_TEST);
      gl.depthMask(!!(material.depthFlags&2));gl.depthFunc(gl.LEQUAL);
      gl.blendFunc((material.alphaFlags>>1&15)===6?gl.SRC_ALPHA:gl.ONE,gl.ONE);
      gl.uniform1f(cutoff,material.threshold/255);gl.bindTexture(gl.TEXTURE_2D,textures.get(material.baseTexture));
      gl.bufferData(gl.ARRAY_BUFFER,new Float32Array(vertices),gl.STREAM_DRAW);
      gl.drawArrays(gl.TRIANGLES,0,vertices.length/9);
     };
     const vertices=billboardVertices(source.billboard,child.elapsed,anchor,view);
     render(source.billboard.material,vertices);
     if(vertices.length)result[kind]++;
     for(let i=0;i<child.emitters.length;i++){
      const rows=child.emitters[i].snapshot();
      if(!rows.length)continue;
      render(source.emitters[i].material,particleVertices(rows,anchor,view));result.particles+=rows.length;
     }
    }
   }
  }finally{restoreSceneGLState(gl,before);}
  return result;
 }
 return {...timeline,draw};
}
