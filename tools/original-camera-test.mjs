import assert from 'node:assert/strict';
import {ORIGINAL_CAMERA_PROFILE as p,sampleOriginalCamera,originalFractionForZoom,updateOriginalCamera,originalCameraRay} from '../docs/original-camera.js';
const near=(a,b,epsilon=1e-7)=>assert.ok(Math.abs(a-b)<=epsilon,`${a} ≠ ${b}`);
let checks=0;
const initial=sampleOriginalCamera(.2);
near(initial.distance,24);near(initial.fov*180/Math.PI,57,1e-5);
// Independent transcription of the actual00520904–0052092e branch, not the
// config header's intended KNEEDIST curve. This catches the native quirk.
near(initial.pitch,p.minPitch+(.2/p.minPitch)*(p.kneePitch-p.minPitch));
near(initial.pitch*180/Math.PI,41.7931,1e-4);checks+=4;
near(sampleOriginalCamera(0).pitch,p.minPitch);
near(sampleOriginalCamera(p.minPitch).pitch,p.kneePitch);
near(sampleOriginalCamera(1).pitch,p.maxPitch);checks+=3;
for(const [width,height] of [[1280,720],[1024,768],[2560,1600],[720,1280]]){
 let previous=Infinity;
 for(let i=0;i<=100;i++){
  const q=i/100,pose=sampleOriginalCamera(q,{width,height});
  assert.ok(pose.zoom<previous);previous=pose.zoom;
  near(originalFractionForZoom(pose.zoom,width,height),q,1e-10);
  assert.ok([pose.distance,pose.pitch,pose.fov,pose.tanX,pose.tanY,pose.zoom].every(Number.isFinite));checks+=3;
 }
 for(const q of [0,.2,.6,1]){
  const cam={x:70,y:80,nativeFraction:q},view={vp:new Float32Array(16)};
  updateOriginalCamera(cam,view,width,height);
  near(view.eye[1],view.distance*Math.sin(view.pitch));
  near(view.eye[2]-cam.y,view.distance*Math.cos(view.pitch));
  near(cam.zoom,view.zoom);checks+=3;
  assert.equal(view.near,1);assert.equal(view.far,300);checks+=2;
  for(const [px,py] of [[width*.1,height*.1],[width*.5,height*.5],[width*.9,height*.9]]){
   const ray=originalCameraRay(view,px,py,width,height);
   const world=[view.eye[0]+ray[0]*15,view.eye[1]+ray[1]*15,view.eye[2]+ray[2]*15,1];
   const clip=[0,0,0,0];for(let row=0;row<4;row++)for(let col=0;col<4;col++)clip[row]+=view.vp[col*4+row]*world[col];
   near((clip[0]/clip[3]+1)*width/2,px,.02);
   near((1-clip[1]/clip[3])*height/2,py,.02);checks+=2;
  }
 }
}
const collisionCam={x:20,y:20,nativeFraction:0},collisionView={};
updateOriginalCamera(collisionCam,collisionView,1280,720,(x,z)=>z>22?10:0);
near(collisionView.eye[1],11);assert.ok(collisionView.pitch>p.minPitch);checks+=2;
const terrainCam={x:20,y:20,nativeFraction:.2},terrainView={};
updateOriginalCamera(terrainCam,terrainView,1280,720,()=>3);
near(terrainView.eye[1],3*.8+24*Math.sin(initial.pitch));checks++;
assert.equal(originalFractionForZoom(Infinity),originalFractionForZoom(NaN));
assert.equal(sampleOriginalCamera(-1).fraction,0);assert.equal(sampleOriginalCamera(2).fraction,1);checks+=3;
console.log(JSON.stringify({suite:'native-camera',checks,passed:true,projectionScope:'Curve, horizontal FOV and near/far defaults verified in Ghidra; north-up coordinates remain the current map adapter.'}));
