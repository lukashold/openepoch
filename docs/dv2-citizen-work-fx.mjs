// Own portable adapter over unsampled source NIF/KF data. The caller owns all
// attachment/event origins, state changes, scheduling, culling and removal.
const compiled = new WeakMap();
const f32 = Math.fround;
const vec = (v, keys='xyz') => [...keys].map(key => v[key]);
const assert = (ok,message) => {if(!ok)throw new Error('Wheat harvest FX: '+message);};
const finite = (array,length) => array?.length===length&&Array.from(array).every(Number.isFinite);
const matrix = r => [r.m11,r.m21,r.m31,r.m12,r.m22,r.m32,r.m13,r.m23,r.m33];
const multiply = (a,b) => Array.from({length:9},(_,i)=>f32([0,1,2].reduce((n,k)=>n+a[Math.floor(i/3)*3+k]*b[k*3+i%3],0)));
const transform = (m,v) => [0,1,2].map(i=>f32(m[i*3]*v[0]+m[i*3+1]*v[1]+m[i*3+2]*v[2]));
const identity = [1,0,0,0,1,0,0,0,1];

function scalar(value){return typeof value==='number'?[value]:vec(value);}

// Linear and quadratic source key groups. Endpoint clamping is a curve-domain
// operation only: this function never wraps elapsed or ends an attachment.
export function sampleWheatKey(group,seconds){
 const keys=group?.Keys;
 if(!keys?.length)return null;
 assert(Number.isFinite(seconds)&&seconds>=0,'invalid supplied curve time');
 let a=keys[0],b=a;
 if(seconds>=keys.at(-1).Time)a=b=keys.at(-1);
 else if(seconds>keys[0].Time){
  const high=keys.findIndex(key=>key.Time>=seconds);a=keys[high-1];b=keys[high];
 }
 const first=scalar(a.Value),last=scalar(b.Value);
 if(a===b||seconds===a.Time)return first.map(f32);
 if(seconds===b.Time)return last.map(f32);
 const alpha=f32((seconds-a.Time)/(b.Time-a.Time));
 if(group.Interpolation===1)return first.map((v,i)=>f32(v+(last[i]-v)*alpha));
 assert(group.Interpolation===2,'unsupported source interpolation');
 const outgoing=scalar(a.Forward),incoming=scalar(b.Backward);
 return first.map((value,i)=>{
  const difference=last[i]-value,cubic=outgoing[i]+incoming[i]-2*difference;
  const square=3*difference-2*outgoing[i]-incoming[i];
  return f32(((cubic*alpha+square)*alpha+outgoing[i])*alpha+value);
 });
}

function euler(x,y,z){
 const [cx,sx,cy,sy,cz,sz]=[Math.cos(x),Math.sin(x),Math.cos(y),Math.sin(y),Math.cos(z),Math.sin(z)];
 return multiply(multiply([cz,-sz,0,sz,cz,0,0,0,1],[cy,0,sy,0,1,0,-sy,0,cy]),[1,0,0,0,cx,-sx,0,sx,cx]);
}

function triangles(data){
 const result=[];
 for(const strip of data.Points??[])for(let i=2;i<strip.length;i++){
  const triangle=i%2?[strip[i-1],strip[i-2],strip[i]]:[strip[i-2],strip[i-1],strip[i]];
  // Removing zero-index-area degenerate strip connectors changes no fragments.
  if(new Set(triangle).size===3)result.push(...triangle);
 }
 return new Uint16Array(result);
}

function transformData(objects,controller){
 if(!controller)return null;
 assert(controller._type==='NiKeyframeController','unsupported rigid controller');
 const data=objects[controller.Data];
 assert(data?._type==='NiKeyframeData','unsupported rigid key layout');
 if(data['Quaternion Keys']?.length)assert(data['Quaternion Keys'].length===1&&
  Object.entries(data['Quaternion Keys'][0].Value).every(([key,value])=>value===(key==='w'?1:0)),
  'only literal source identity quaternion accepted');
 else if(data['Num Rotation Keys'])assert(data['Rotation Type']===4,'expected source XYZ rotations');
 return data;
}

export function compileWheatHarvest(input){
 assert(input?.format==='ee2-unsampled-wheat-harvest-v1','unsupported source graph');
 const asset=structuredClone(input),objects=asset.nif.objects,kf=asset.kf.objects;
 assert(objects.length===127&&kf.length===70,'source graph shape differs');
 for(const list of [objects,kf])for(let i=0;i<list.length;i++)assert(list[i]._id===i,'source indices differ');
 const parent=new Map(),nodes=new Map(),properties=new Map();
 for(const node of objects)for(const child of node.Children??[]){assert(!parent.has(child),'shared/ambiguous scene node');parent.set(child,node._id);}
 function propertiesFor(node){
  if(properties.has(node._id))return properties.get(node._id);
  const inherited=parent.has(node._id)?{...propertiesFor(objects[parent.get(node._id)])}:{};
  for(const ref of node.Properties??[])inherited[objects[ref]._type]=objects[ref];
  properties.set(node._id,inherited);return inherited;
 }
 for(const node of objects.filter(row=>row.Translation)){
  assert(finite(vec(node.Translation),3)&&finite(matrix(node.Rotation),9)&&Number.isFinite(node.Scale),'invalid source transform');
  nodes.set(node._id,{node,parent:parent.get(node._id),embedded:transformData(objects,node.Controller<0?null:objects[node.Controller])});
 }
 const sequences=kf.filter(row=>row._type==='NiControllerSequence');assert(sequences.length===1,'ambiguous external sequence');
 const byName=new Map([...nodes.values()].map(row=>[row.node.Name,row]));
 const external=new Map(),colorChannels=[],unboundIdentityTargets=[];
 for(const block of sequences[0]['Controlled Blocks']){
  const controller=kf[block.Controller];
  if(controller._type==='NiKeyframeController'){
   const target=byName.get(block['Target Name']),data=transformData(kf,controller);
   if(!target){
    assert(block['Target Name']==='Dummy01_harvest NonAccum'&&data['Quaternion Keys']?.length===1&&
     data.Translations.Keys.length===1&&vec(data.Translations.Keys[0].Value).every(v=>v===0)&&
     data.Scales.Keys.length===1&&data.Scales.Keys[0].Value===1,'nonidentity unbound KF target');
    unboundIdentityTargets.push(block['Target Name']);continue;
   }
   assert(!external.has(target.node._id),'duplicate KF target');external.set(target.node._id,data);
  }else{
   assert(controller._type==='NiMaterialColorController','unsupported KF controller');
   const [name,prop,kind]=block['Target Name'].split('\n');
   assert(/^Editable Mesh(?:\d+)?$/.test(name)&&prop==='PROP'&&kind==='NiMaterialProperty','unexpected source colour target');
   const data=kf[controller.Data];assert(data?._type==='NiPosData','invalid source colour knots');
   assert(data.Data.Keys.every(key=>vec(key.Value).every(v=>v===1)&&vec(key.Forward).every(v=>v===0)&&vec(key.Backward).every(v=>v===0)),
    'source colour invariant no longer holds; alias resolution requires native proof');
   colorChannels.push({targetName:block['Target Name'],keys:data.Data,controller});
  }
 }
 const parts=objects.filter(row=>row._type==='NiTriStrips').map(node=>{
  const data=objects[node.Data],props=propertiesFor(node),uvSet=props.NiTexturingProperty['Base Texture']['UV Set'];
  assert(data['Num Vertices']===8&&data.Vertices.length===8&&data['Vertex Colors'].length===8,'source wheat geometry differs');
  assert(props.NiAlphaProperty.Flags===4845&&props.NiAlphaProperty.Threshold===0,'source alpha differs');
  assert(props.NiVertexColorProperty['Vertex Mode']===1&&props.NiVertexColorProperty['Lighting Mode']===0,'source unlit vertex-emissive material differs');
  assert(props.NiTexturingProperty['Apply Mode']===2,'source modulate differs');
  const material=props.NiMaterialProperty,controller=objects[material.Controller];
  const embeddedColor=controller?objects[controller.Data].Data:null;
  return {node:node._id,name:node.Name,nativePositions:data.Vertices.map(row=>vec(row)),
   uvs:new Float32Array(data['UV Sets'][uvSet].flatMap(row=>[row.u,row.v])),
   colors:new Float32Array(data['Vertex Colors'].flatMap(row=>vec(row,'rgba'))),
   indices:triangles(data),properties:props,material,embeddedColor,kfColor:colorChannels[0]};
 });
 assert(parts.length===16&&colorChannels.length===16,'expected all16 source stems and colour channels');
 const result=Object.freeze({format:asset.format,parts:Object.freeze(parts),texture:Object.freeze(asset.texture),
  duration:asset.metadata.duration,streams:Object.freeze(['embedded','kf']),
  unboundIdentityTargets:Object.freeze(unboundIdentityTargets),
  kfColorTargets:Object.freeze(colorChannels.map(row=>row.targetName)),
  colorPolicy:'All16 external tracks are constant RGB1/zero tangents; duplicate leaf-name alias resolution is not inferred.',
  precision:'portable-nearest-f32-channel-matrix-storage',
  boundaries:Object.freeze(['Caller supplies the selected source stream and local elapsed; no loop, origin, lifetime or visibility is inferred.',
   'XYZ rotation follows the established portable Rz*Ry*Rx convention; original full XYZ native matrix/trigonometric rounding is not claimed.'])});
 compiled.set(result,{asset,objects,nodes,external});return result;
}

export function sampleWheatHarvest(model,instance){
 const source=compiled.get(model);assert(source,'compiled source required');
 const {position,yaw,scale,elapsed,precision,stream}=instance??{};
 assert(precision==='nearest','explicit portable nearest precision required');
 assert(stream==='embedded'||stream==='kf','explicit selected source stream required');
 assert(finite(position,3)&&Number.isFinite(yaw)&&Number.isFinite(scale)&&scale>0,'invalid supplied attachment transform');
 assert(Number.isFinite(elapsed)&&elapsed>=0&&f32(elapsed)===elapsed,'supplied local elapsed must be a nonnegative f32');
 const worlds=new Map(),visiting=new Set();
 function world(index){
  if(worlds.has(index))return worlds.get(index);
  assert(!visiting.has(index),'cyclic source ancestry');visiting.add(index);
  const descriptor=source.nodes.get(index);assert(descriptor,'missing source transform');
  const node=descriptor.node,data=stream==='embedded'?descriptor.embedded:source.external.get(index);
  let rotation=matrix(node.Rotation),size=node.Scale,translation=vec(node.Translation);
  if(data){
   if(data['Quaternion Keys']?.length)rotation=identity;
   else if(data['Num Rotation Keys'])rotation=euler(...data['XYZ Rotations'].map(group=>sampleWheatKey(group,elapsed)[0]));
   size=sampleWheatKey(data.Scales,elapsed)?.[0]??size;
   translation=sampleWheatKey(data.Translations,elapsed)??translation;
  }
  rotation=rotation.map(value=>f32(value*size));
  const owner=descriptor.parent===undefined?{rotation:identity,translation:[0,0,0]}:world(descriptor.parent);
  const result={rotation:multiply(owner.rotation,rotation),translation:transform(owner.rotation,translation).map((v,i)=>f32(v+owner.translation[i]))};
  visiting.delete(index);worlds.set(index,result);return result;
 }
 const c=Math.cos(yaw),s=Math.sin(yaw);
 const parts=model.parts.map(part=>{
  const pose=world(part.node),positions=new Float32Array(24);
  for(let i=0;i<part.nativePositions.length;i++){
   const native=transform(pose.rotation,part.nativePositions[i]).map((v,j)=>f32(v+pose.translation[j]));
   const [x,y,z]=[native[0],native[2],-native[1]].map(v=>v*scale);
   positions.set([f32(position[0]+x*c-z*s),f32(position[1]+y),f32(position[2]+x*s+z*c)],i*3);
  }
  const colorKeys=stream==='embedded'?part.embeddedColor:part.kfColor.keys;
  const emissive=sampleWheatKey(colorKeys,elapsed)??vec(part.material['Emissive Color'],'rgb');
  return {node:part.node,name:part.name,positions,uvs:part.uvs,colors:part.colors,indices:part.indices,
   emissive,alpha:part.material.Alpha,properties:part.properties};
 });
 return {elapsed,stream,precision,parts};
}

const FX_VS=`#version 300 es
layout(location=0) in vec3 aPosition;
layout(location=1) in vec2 aUV;
layout(location=2) in vec4 aColor;
uniform mat4 uVP;
out vec2 vUV;out vec4 vColor;out vec3 vWorld;
void main(){vUV=aUV;vColor=aColor;vWorld=aPosition;gl_Position=uVP*vec4(aPosition,1.);}`;
const FX_FS=`#version 300 es
precision highp float;
uniform sampler2D uTexture;
uniform vec3 uFXEmissive;
uniform float uFXAlpha;
uniform bool uFXClipEnabled;
uniform vec4 uFXClipPlane;
in vec2 vUV;in vec4 vColor;in vec3 vWorld;
out vec4 frag;
void main(){
 if(uFXClipEnabled&&dot(vec4(vWorld,1.),uFXClipPlane)<0.)discard;
 vec4 value=texture(uTexture,vUV)*vColor;
 value.rgb*=uFXEmissive;value.a*=uFXAlpha;
 if(value.a<=0.)discard;
 frag=value;
}`;

function saveFXState(gl){
 const get=name=>gl.getParameter(gl[name]),active=get('ACTIVE_TEXTURE');
 gl.activeTexture(gl.TEXTURE0);const texture=get('TEXTURE_BINDING_2D');gl.activeTexture(active);
 return {active,texture,program:get('CURRENT_PROGRAM'),vao:get('VERTEX_ARRAY_BINDING'),array:get('ARRAY_BUFFER_BINDING'),
  depth:gl.isEnabled(gl.DEPTH_TEST),blend:gl.isEnabled(gl.BLEND),write:get('DEPTH_WRITEMASK'),depthFunc:get('DEPTH_FUNC'),
  blendFunc:['BLEND_SRC_RGB','BLEND_DST_RGB','BLEND_SRC_ALPHA','BLEND_DST_ALPHA'].map(get),
  blendEquation:['BLEND_EQUATION_RGB','BLEND_EQUATION_ALPHA'].map(get)};
}
function restoreFXState(gl,state){
 gl.useProgram(state.program);gl.bindVertexArray(state.vao);gl.bindBuffer(gl.ARRAY_BUFFER,state.array);
 gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,state.texture);gl.activeTexture(state.active);
 for(const [flag,on]of [[gl.DEPTH_TEST,state.depth],[gl.BLEND,state.blend]])(on?gl.enable:gl.disable).call(gl,flag);
 gl.depthMask(state.write);gl.depthFunc(state.depthFunc);gl.blendFuncSeparate(...state.blendFunc);gl.blendEquationSeparate(...state.blendEquation);
}

// Optional WebGL adapter for this actual source material. Texture decoding and
// verification stay with the caller. Caller culling, fog, lifetime and passes
// are not inferred; native full render-list ordering is not claimed here.
export function createWheatHarvestRenderer(gl,{model,texture,precision}={}){
 assert(compiled.has(model)&&texture&&precision==='nearest','verified compiled source/texture/explicit precision required');
 const before=saveFXState(gl),cleanup=[];let program,disposed=false;
 try{
  program=gl.createProgram();cleanup.push(()=>gl.deleteProgram(program));
  for(const [type,source]of [[gl.VERTEX_SHADER,FX_VS],[gl.FRAGMENT_SHADER,FX_FS]]){
   const shader=gl.createShader(type);gl.shaderSource(shader,source);gl.compileShader(shader);
   assert(gl.getShaderParameter(shader,gl.COMPILE_STATUS),gl.getShaderInfoLog(shader));gl.attachShader(program,shader);gl.deleteShader(shader);
  }
  gl.linkProgram(program);assert(gl.getProgramParameter(program,gl.LINK_STATUS),gl.getProgramInfoLog(program));
  const uniforms=Object.fromEntries(['uVP','uTexture','uFXEmissive','uFXAlpha','uFXClipEnabled','uFXClipPlane'].map(name=>[name,gl.getUniformLocation(program,name)]));
  const geometries=model.parts.map(part=>{
   const vao=gl.createVertexArray();cleanup.push(()=>gl.deleteVertexArray(vao));gl.bindVertexArray(vao);
   const buffer=(location,size,data,usage)=>{const value=gl.createBuffer();cleanup.push(()=>gl.deleteBuffer(value));gl.bindBuffer(gl.ARRAY_BUFFER,value);gl.bufferData(gl.ARRAY_BUFFER,data,usage);gl.enableVertexAttribArray(location);gl.vertexAttribPointer(location,size,gl.FLOAT,false,0,0);return value;};
   const positions=buffer(0,3,new Float32Array(24),gl.STREAM_DRAW);
   buffer(1,2,part.uvs,gl.STATIC_DRAW);buffer(2,4,part.colors,gl.STATIC_DRAW);
   const indices=gl.createBuffer();cleanup.push(()=>gl.deleteBuffer(indices));gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,indices);gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,part.indices,gl.STATIC_DRAW);
   return {vao,positions,count:part.indices.length};
  });
  return {
   draw(instances,{vp,clipPlane=null}={}){
    assert(!disposed&&finite(vp,16)&&(!clipPlane||finite(clipPlane,4)),'invalid renderer state/VP/clip plane');
    const state=saveFXState(gl);let calls=0;
    try{
     gl.useProgram(program);gl.uniformMatrix4fv(uniforms.uVP,false,vp);
     gl.uniform1i(uniforms.uTexture,0);gl.uniform1i(uniforms.uFXClipEnabled,clipPlane?1:0);gl.uniform4fv(uniforms.uFXClipPlane,clipPlane??[0,0,0,0]);
     gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,texture);
     gl.enable(gl.BLEND);gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);gl.blendEquation(gl.FUNC_ADD);
     for(const instance of instances){
      if(instance.visible===false)continue;
      const sample=sampleWheatHarvest(model,{...instance,precision});
      for(let i=0;i<sample.parts.length;i++){
       const part=sample.parts[i],gpu=geometries[i],depth=part.properties.NiZBufferProperty;
       assert(depth&&depth.Function===3,'unsupported source depth comparison');
       (depth.Flags&1?gl.enable:gl.disable).call(gl,gl.DEPTH_TEST);gl.depthMask(!!(depth.Flags&2));gl.depthFunc(gl.LEQUAL);
       gl.uniform3fv(uniforms.uFXEmissive,part.emissive);gl.uniform1f(uniforms.uFXAlpha,part.alpha);
       gl.bindVertexArray(gpu.vao);gl.bindBuffer(gl.ARRAY_BUFFER,gpu.positions);gl.bufferData(gl.ARRAY_BUFFER,part.positions,gl.STREAM_DRAW);
       gl.drawElements(gl.TRIANGLES,gpu.count,gl.UNSIGNED_SHORT,0);calls++;
      }
     }
     return calls;
    }finally{restoreFXState(gl,state);}
   },
   dispose(){if(!disposed){disposed=true;for(const remove of cleanup.reverse())remove();}},
  };
 }catch(error){for(const remove of cleanup.reverse())remove();throw error;}
 finally{restoreFXState(gl,before);}
}
