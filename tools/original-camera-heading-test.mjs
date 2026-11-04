import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {ORIGINAL_CAMERA_PROFILE as profile,sampleOriginalCamera,updateOriginalCamera,originalCameraRay,originalCameraRaySegment,
 normalizeOriginalHeading,rotateOriginalCameraHeading,applyOriginalCameraNativeTurn,originalCameraHeadingAxes} from '../docs/original-camera.js';

const research=new URL('../../EE2-Research/analysis/',import.meta.url),root=new URL('../../../',import.meta.url);
const sha=b=>createHash('sha256').update(b).digest('hex');
let checks=0,northCases=0,rotationCases=0;
function equal(a,b,label){assert.equal(a,b,label);checks++;}
function close(a,b,label,eps=1e-10){assert.ok(Math.abs(a-b)<=eps,`${label}: ${a} != ${b}`);checks++;}
function exact(a,b,label){
 if(Array.isArray(a)||ArrayBuffer.isView(a)){equal(a.length,b.length,label+' length');for(let i=0;i<a.length;i++)exact(a[i],b[i],label+'['+i+']');}
 else{assert.ok(Object.is(a,b),`${label}: ${a} != ${b} (including signed zero)`);checks++;}
}
const dot=(a,b)=>a.reduce((sum,x,i)=>sum+x*b[i],0);
const clip=(view,point)=>[0,1,2,3].map(row=>view.vp[row]*point[0]+view.vp[4+row]*point[1]+view.vp[8+row]*point[2]+view.vp[12+row]);
// Independent transcription of00520600: native matrix columns D/U/R. Native
// worldXYZ maps to browserX/Z/-Y. These are not read from the implementation.
function nativeBasis(pitch,heading){
 const half=profile.nativeDefaultHeading,pitchCos=Math.cos(half+pitch),pitchSin=Math.sin(half+pitch);
 const sourceHeading=half+heading,cos=Math.cos(sourceHeading-half),sin=Math.sin(sourceHeading-half);
 const matrix=[-sin*pitchSin,sin*pitchCos,cos, cos*pitchSin,-cos*pitchCos,sin, pitchCos,pitchSin,0];
 const adapt=([x,y,z])=>[x,z,-y];
 return {f:adapt([matrix[0],matrix[3],matrix[6]]),u:adapt([matrix[1],matrix[4],matrix[7]]),right:adapt([matrix[2],matrix[5],matrix[8]])};
}

const baselinePath=new URL('work/heading-baseline/original-camera.js',root);
let baselineSha256=null;
if(fs.existsSync(baselinePath)){
 const bytes=fs.readFileSync(baselinePath);baselineSha256=sha(bytes);
 const old=await import('data:text/javascript;base64,'+bytes.toString('base64'));
 for(const [width,height]of [[1280,720],[2560,1600],[720,1280],[NaN,0]])for(const fraction of [0,.01,.2,profile.minPitch,.6,.99,1,NaN])
 for(const [x,y]of [[70,80],[-0,-0],[-21,43]])for(const heights of [()=>0,()=>3,(x,z)=>z>22?10:0]){
  const a={x,y,nativeFraction:fraction},b={...a},av={vp:new Float32Array(16)},bv={vp:new Float32Array(16)};
  old.updateOriginalCamera(a,av,width,height,heights);updateOriginalCamera(b,bv,width,height,heights);
  for(const k of Object.keys(av))exact(av[k],bv[k],'north '+k);
  exact(a.zoom,b.zoom,'north UI zoom');exact(a.nativeFraction,b.nativeFraction,'north zoom fraction');
  for(const [px,py]of [[0,0],[400,300],[900,600]])exact(old.originalCameraRay(av,px,py,width,height),originalCameraRay(bv,px,py,width,height),'north ray');
  northCases++;
 }
}
for(const value of [0,-0,2*Math.PI,-2*Math.PI,Infinity,NaN])equal(normalizeOriginalHeading(value),0,'heading wraps/sanitizes to exact north');
close(normalizeOriginalHeading(5*Math.PI/2),Math.PI/2,'positive wrap');close(normalizeOriginalHeading(-5*Math.PI/2),-Math.PI/2,'negative wrap');
const rotation={};close(rotateOriginalCameraHeading(rotation,Math.PI/4),Math.PI/4,'positive API turn');close(applyOriginalCameraNativeTurn(rotation,Math.PI/2),-Math.PI/4,'native orbit delta sign');
equal(rotateOriginalCameraHeading(rotation,NaN),rotation.heading,'invalid delta does not turn');
for(const [width,height]of [[1280,720],[1024,768],[720,1280]])for(const q of [0,.2,profile.minPitch,.6,1])
for(const heading of [0,Math.PI/4,Math.PI/2,Math.PI,-Math.PI/4,-Math.PI/2,5*Math.PI/4]){
 const cam={x:70,y:80,nativeFraction:q,heading},view={};updateOriginalCamera(cam,view,width,height);
 const pose=sampleOriginalCamera(q,{width,height,heading}),h=normalizeOriginalHeading(heading),basis=nativeBasis(view.pitch,h);
 close(view.eye[0]-cam.x,Math.sin(h)*pose.distance*Math.cos(pose.pitch),'native orbit eyeX',1e-12);
 close(view.eye[2]-cam.y,Math.cos(h)*pose.distance*Math.cos(pose.pitch),'native orbit eyeZ',1e-12);
 close(view.eye[1],pose.distance*Math.sin(pose.pitch),'orbit altitude');
 for(const key of ['f','u','right'])for(let i=0;i<3;i++)close(view[key][i],basis[key][i],'00520600 '+key,1e-7);
 for(const key of ['f','u','right'])close(dot(view[key],view[key]),1,key+' unit');
 close(dot(view.f,view.u),0,'forward/up orthogonal');close(dot(view.right,view.u),0,'right/up orthogonal');close(dot(view.right,view.f),0,'right/forward orthogonal');
 const axes=originalCameraHeadingAxes(heading);close(dot(axes.groundRight,axes.groundForward),0,'map pan axes orthogonal');
 close(Math.hypot(...axes.groundRight),1,'map right unit');close(Math.hypot(...axes.groundForward),1,'map forward unit');
 equal(view.near,1,'native near');equal(view.far,300,'native far');close(view.zoom,sampleOriginalCamera(q,{width,height}).zoom,'heading does not change UI zoom');
 for(const [px,py]of [[0,0],[width*.1,height*.1],[width/2,height/2],[width*.9,height*.9],[width,height]]){
  const ray=originalCameraRay(view,px,py,width,height),segment=originalCameraRaySegment(view,px,py,width,height);
  close(dot(view.f,ray),1,'ray parameter is forward depth');
  for(const [distance,expectedZ]of [[view.near,-1],[15,null],[view.far,1]]){
   const point=view.eye.map((v,i)=>v+ray[i]*distance),c=clip(view,point);
   close((c[0]/c[3]+1)*width/2,px,'heading ray/project x',.04);
   close((1-c[1]/c[3])*height/2,py,'heading ray/project y',.04);
   if(expectedZ!==null)close(c[2]/c[3],expectedZ,'clip endpoint',.00006);
  }
  for(let i=0;i<3;i++){close(segment.nearPoint[i],segment.origin[i]+segment.direction[i]*view.near,'near endpoint');close(segment.farPoint[i],segment.origin[i]+segment.direction[i]*view.far,'far endpoint');}
 }
 rotationCases++;
}
// Terrain collision must sample the rotated eye position, then retain the same
// native correction pitch and heading axes rather than looking at the focus.
for(const heading of [Math.PI/4,Math.PI/2,Math.PI,-Math.PI/2]){
 const cam={x:20,y:20,nativeFraction:0,heading},flat={};updateOriginalCamera({...cam},flat,1280,720);
 const calls=[],terrain=(x,z)=>{calls.push([x,z]);return Math.hypot(x-flat.eye[0],z-flat.eye[2])<.01?10:0;},view={};
 updateOriginalCamera(cam,view,1280,720,terrain);close(view.eye[1],11,'rotated terrain clearance');
 close(calls[1][0],flat.eye[0],'terrain at rotated eye x');close(calls[1][1],flat.eye[2],'terrain at rotated eye z');
 close(view.pitch,Math.atan2(11,profile.minDistance*Math.cos(profile.minPitch)),'native terrain correction pitch');
 const center=originalCameraRay(view,640,360,1280,720);for(let i=0;i<3;i++)close(center[i],view.f[i],'corrected center ray');
}

// Real GLES consumes these production module matrices. Its point positions come
// from the public picking rays; expected pixels are the independent input pixels.
const gpuCases=[];
for(const heading of [0,Math.PI/4,Math.PI/2,Math.PI])for(const fraction of [0,.2,.7,1]){
 const width=256,height=256,cam={x:64,y:64,nativeFraction:fraction,heading},view={};updateOriginalCamera(cam,view,width,height);
 const samples=[[32,64,15,true],[128,128,15,true],[220,196,15,true],[220,64,1.01,true],[32,196,299,true],[64,128,.99,false],[192,128,301,false],[128,32,-15,false]].map(([px,py,depth,visible])=>{
  const ray=originalCameraRay(view,px,py,width,height);return {pixel:[px,py],depth,visible,world:view.eye.map((v,i)=>v+ray[i]*depth)};
 });
 gpuCases.push({heading,fraction,width,height,vp:Array.from(view.vp),samples});
}
const source=fs.readFileSync(new URL('../docs/original-camera.js',import.meta.url));
const report={format:'native-camera-heading-tests-v1',passed:true,checks,northCases,rotationCases,baselineSha256,cameraSha256:sha(source),
 scope:'North baseline bit equality, independent native00520600 basis, heading orbit/pan, rotated terrain clearance, camera-depth ray endpoints and projection. UI input rate remains an adapter.',gpuCases:gpuCases.length};
fs.writeFileSync(new URL('native-camera-heading-tests.json',research),JSON.stringify(report,null,2)+'\n');
fs.writeFileSync(new URL('native-camera-heading-fixtures.json',research),JSON.stringify({cameraSha256:sha(source),cases:gpuCases})+'\n');
console.log(JSON.stringify(report));
