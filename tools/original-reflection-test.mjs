// Production reflection helper, camera mathematics and exceptional GL-state paths.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {RecordingGL} from './recording-webgl.mjs';
import {createOriginalReflection,reflectedViewProjection} from '../docs/original-reflection.js';
import {updateOriginalCamera} from '../docs/original-camera.js';
let checks=0;const check=(condition,message)=>{assert.ok(condition,message);checks++;};
const equal=(a,b,message)=>{assert.deepEqual(a,b,message);checks++;};
const rejects=(fn,pattern)=>{assert.throws(fn,pattern);checks++;};
const project=(m,p)=>[0,1,2,3].map(row=>m[row]*p[0]+m[4+row]*p[1]+m[8+row]*p[2]+m[12+row]);
for(const height of [0,.012,2,-3.5])for(const heading of [0,.7,-1.3]){
 const view={vp:new Float32Array(16)},cam={x:35,y:42,nativeFraction:.35,heading};updateOriginalCamera(cam,view,1280,720,()=>0);
 const mirrored=reflectedViewProjection(view.vp,height);
 for(let i=0;i<31;i++){
  const p=[i*.71,i*.13-2,50-i*.21],a=project(mirrored,p),b=project(view.vp,[p[0],2*height-p[1],p[2]]);
  check(a.every((value,index)=>Math.abs(value-b[index])<.00008),'VP reflection preserves world-plane geometry');
 }
 const twice=reflectedViewProjection(reflectedViewProjection(view.vp,height),height);
 check(Array.from(view.vp).every((value,index)=>Math.abs(value-twice[index])<.00002),'Double reflection restores original VP within float32 rounding');
}
for(const args of [[null,0],[new Float32Array(15),0],[new Float32Array(16),Infinity]])rejects(()=>reflectedViewProjection(...args),/Invalid reflection camera/);

const gl=new RecordingGL();
const framebuffer=gl.createFramebuffer(),readFramebuffer=gl.createFramebuffer(),renderbuffer=gl.createRenderbuffer();
gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER,framebuffer);gl.bindFramebuffer(gl.READ_FRAMEBUFFER,readFramebuffer);gl.bindRenderbuffer(gl.RENDERBUFFER,renderbuffer);
gl.viewport(12,34,567,321);gl.scissor(7,8,90,100);gl.enable(gl.SCISSOR_TEST);gl.enable(gl.CULL_FACE);gl.enable(gl.BLEND);gl.enable(gl.STENCIL_TEST);gl.disable(gl.DEPTH_TEST);
gl.clearColor(.1,.2,.3,.4);gl.clearDepth(.6);gl.clearStencil(5);gl.colorMask(true,false,true,false);gl.depthMask(false);gl.depthFunc(gl.GEQUAL);gl.stencilMask(91);
gl.stencilMaskSeparate(gl.BACK,23);
gl.frontFace(gl.CW);gl.cullFace(gl.BACK);gl.blendFuncSeparate(gl.SRC_ALPHA,gl.ONE,gl.DST_ALPHA,gl.ZERO);gl.blendEquationSeparate(gl.FUNC_ADD,gl.FUNC_ADD);
gl.useProgram(gl.createProgram());gl.bindVertexArray(gl.createVertexArray());gl.bindBuffer(gl.ARRAY_BUFFER,gl.createBuffer());
for(let unit=0;unit<16;unit++){gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(gl.TEXTURE_2D,gl.createTexture());gl.bindTexture(gl.TEXTURE_2D_ARRAY,gl.createTexture());}gl.activeTexture(gl.TEXTURE0+9);
function fingerprint(){
 const fields=['ACTIVE_TEXTURE','DRAW_FRAMEBUFFER_BINDING','READ_FRAMEBUFFER_BINDING','RENDERBUFFER_BINDING','VIEWPORT','SCISSOR_BOX','COLOR_CLEAR_VALUE','DEPTH_CLEAR_VALUE','STENCIL_CLEAR_VALUE','COLOR_WRITEMASK','DEPTH_WRITEMASK','DEPTH_FUNC','STENCIL_WRITEMASK','STENCIL_BACK_WRITEMASK','FRONT_FACE','CULL_FACE_MODE','CURRENT_PROGRAM','VERTEX_ARRAY_BINDING','ARRAY_BUFFER_BINDING','BLEND_SRC_RGB','BLEND_DST_RGB','BLEND_SRC_ALPHA','BLEND_DST_ALPHA','BLEND_EQUATION_RGB','BLEND_EQUATION_ALPHA'];
 const result=Object.fromEntries(fields.map(name=>[name,gl.getParameter(gl[name])]));
 result.flags=[...gl.enabled].sort();result.bindings=[...gl.units].filter(([,value])=>value).map(([key,value])=>[key,value.id]).sort();return structuredClone(result);
}
const initial=fingerprint(),vp=new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]);
const renderer=createOriginalReflection(gl,{resolutionScale:.5,maxSize:512});let callbacks=0;
const pass=renderer.render({vp,eye:[4,10,8],width:1280,height:720,waterHeight:2,drawScene:args=>{
 callbacks++;equal(args.eye,[4,-6,8]);equal(args.clipPlane,[0,1,0,-2]);check(args.capture===true);
 equal(gl.getParameter(gl.VIEWPORT),[0,0,512,288]);check(gl.isEnabled(gl.DEPTH_TEST));check(!gl.isEnabled(gl.SCISSOR_TEST));check(!gl.isEnabled(gl.CULL_FACE));
 const target=gl.drawFramebuffer.attachments[gl.COLOR_ATTACHMENT0].texture;
 check(![...gl.units.values()].some(value=>value?.id===target),'No sampler feedback from allocated target');
 gl.activeTexture(gl.TEXTURE0+3);gl.bindTexture(gl.TEXTURE_2D,gl.createTexture());gl.bindTexture(gl.TEXTURE_2D_ARRAY,null);gl.enable(gl.BLEND);gl.depthMask(false);
}});
equal(callbacks,1);equal(fingerprint(),initial,'All caller state restored after capture');equal([pass.width,pass.height],[512,288]);equal(renderer.metrics,{frames:1,resizes:1,width:512,height:288});
// Simulate the prior water pass retaining this texture before the next capture.
gl.activeTexture(gl.TEXTURE0+7);gl.bindTexture(gl.TEXTURE_2D,pass.texture);const previous=fingerprint();
renderer.render({vp,eye:[4,10,8],width:1280,height:720,drawScene:()=>{check(![...gl.units.values()].some(value=>value===pass.texture),'Previous reflection target removed before drawing');}});
equal(fingerprint(),previous,'Prior water texture binding restored');equal(renderer.metrics.resizes,1,'Same-size captures reuse attachments');
const resized=renderer.render({vp,eye:[4,10,8],width:800,height:400,drawScene:()=>{}});equal([resized.width,resized.height],[400,200]);equal(renderer.metrics.resizes,2);equal(fingerprint(),previous,'Resize retains still-bound prior target and restores caller');
for(const drawScene of [()=>{throw Error('fixture failure');},()=>Promise.resolve(),()=>renderer.render({vp,eye:[4,10,8],width:800,height:400,drawScene(){}})]){
 const before=fingerprint();rejects(()=>renderer.render({vp,eye:[4,10,8],width:800,height:400,drawScene}),/fixture failure|must be synchronous|Recursive/);equal(fingerprint(),before,'Exceptional capture restores framebuffer, viewport and flags');
}
const status=gl.checkFramebufferStatus;gl.checkFramebufferStatus=()=>0;
rejects(()=>renderer.render({vp,eye:[4,10,8],width:500,height:500,drawScene(){}}),/Incomplete/);equal(fingerprint(),previous,'Allocation failure restores state');gl.checkFramebufferStatus=status;
for(const options of [{resolutionScale:0},{resolutionScale:Infinity},{resolutionScale:3},{maxSize:0}])rejects(()=>createOriginalReflection(gl,options),/Invalid/);
for(const options of [{eye:[0,NaN,0]},{width:0},{height:NaN},{drawScene:null},{waterHeight:NaN}])rejects(()=>renderer.render({vp,eye:[4,10,8],width:800,height:400,drawScene(){},...options}),/Invalid/);
renderer.dispose();rejects(()=>renderer.render({vp,eye:[4,10,8],width:800,height:400,drawScene(){}}),/disposed/);
const sha=async path=>createHash('sha256').update(await fs.readFile(new URL(path,import.meta.url))).digest('hex');
const report={format:'openepoch-original-reflection-contract-tests-v1',passed:true,checks,
 sourceHashes:{'original-reflection.js':await sha('../docs/original-reflection.js'),'original-terrain.js':await sha('../docs/original-terrain.js'),'original-camera.js':await sha('../docs/original-camera.js')},
 coverage:['mirrored VP geometry and involution','actual native-camera matrices','proportional resize/attachment reuse','distinct caller draw/read framebuffer restoration','viewport/scissor/color/depth/stencil/cull/blend/program/VAO/buffer/16 texture slots restoration','previous-frame reflection feedback prevention','recursive/async/exception/incomplete-FBO validation'],
 scope:'Actual production helper with recording WebGL. Separate real-GLES fixture/world replay validates shader pixels and framebuffer attachments; no browser or native executable.'};
await fs.writeFile(new URL('../../EE2-Research/analysis/native-reflection-contract-tests.json',import.meta.url),JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report));
