// Native orbit curve: EE2-engine.exe SHA d41a9e15…e6455f, FUN_005207f0.
// Config parser0051d090 and installed myconfig.cfg supply the values below.
// Horizontal FOV/frustum:0051f9f0. Near/far:0051e8a0→0051f810.
// North-up coordinate axes adapt this camera to the current simulation map;
// later launcher hooks and camera rotation controls are outside this profile.
const RAD=Math.fround(0.01745329238474369);
const clamp=(value,min,max)=>Math.min(max,Math.max(min,value));
const finite=(value,fallback)=>Number.isFinite(value)?value:fallback;
export const ORIGINAL_CAMERA_PROFILE=Object.freeze({
 minDistance:5,maxDistance:100,defaultFraction:.2,minHeight:1,
 minPitch:Math.fround(22*RAD),kneePitch:Math.fround(60*RAD),maxPitch:Math.fround(80*RAD),
 // The parser stores this value; the observed update reads minPitch instead.
 parsedKneeDistance:Math.fround(14.32*RAD),
 minFov:Math.fround(55*RAD),maxFov:Math.fround(65*RAD),
 fovAxis:'horizontal',near:1,far:300,
});
function dimensions(width,height){return [Math.max(1,finite(width,1280)),Math.max(1,finite(height,720))];}

export function sampleOriginalCamera(fraction,{width=1280,height=720,profile=ORIGINAL_CAMERA_PROFILE}={}){
 [width,height]=dimensions(width,height);
 const q=clamp(finite(fraction,profile.defaultFraction),0,1);
 // Preserve the compiled original's use of +0x3c for both angle and knee
 // threshold. Replacing it with the documented KNEEDIST changes the camera.
 const knee=profile.minPitch;
 const pitch=q<knee?profile.minPitch+(profile.kneePitch-profile.minPitch)*q/knee:
  profile.kneePitch+(profile.maxPitch-profile.kneePitch)*(q-knee)/(1-knee);
 const distance=profile.minDistance+(profile.maxDistance-profile.minDistance)*q;
 const fov=profile.minFov+(profile.maxFov-profile.minFov)*q;
 const tangent=Math.tan(fov/2),aspect=width/height;
 const tanY=profile.fovAxis==='horizontal'?tangent/aspect:tangent;
 const tanX=tanY*aspect;
 return {fraction:q,distance,pitch,fov,tanX,tanY,zoom:height/(2*distance*tanY)};
}

export function originalFractionForZoom(zoom,width=1280,height=720,profile=ORIGINAL_CAMERA_PROFILE){
 const nearest=sampleOriginalCamera(0,{width,height,profile}).zoom;
 const furthest=sampleOriginalCamera(1,{width,height,profile}).zoom;
 const desired=clamp(finite(zoom,sampleOriginalCamera(profile.defaultFraction,{width,height,profile}).zoom),furthest,nearest);
 if(desired===nearest)return 0;if(desired===furthest)return 1;
 let lo=0,hi=1;
 for(let i=0;i<40;i++){const q=(lo+hi)/2;if(sampleOriginalCamera(q,{width,height,profile}).zoom>desired)lo=q;else hi=q;}
 return (lo+hi)/2;
}

/** Update the existing front-end cam/view contract without touching simulation. */
export function updateOriginalCamera(cam,view,width,height,heightAt=()=>0,profile=ORIGINAL_CAMERA_PROFILE){
 [width,height]=dimensions(width,height);
 const pose=sampleOriginalCamera(cam.nativeFraction,{width,height,profile});
 cam.nativeFraction=pose.fraction;cam.zoom=pose.zoom;
 const x=finite(cam.x,0),z=finite(cam.y,0),targetHeight=Math.max(0,finite(heightAt(x,z),0));
 const horizontal=pose.distance*Math.cos(pose.pitch);
 const eye=[x,(1-pose.fraction)*targetHeight+pose.distance*Math.sin(pose.pitch),z+horizontal];
 const floor=Math.max(0,finite(heightAt(eye[0],eye[2]),0))+profile.minHeight;
 let pitch=pose.pitch;
 if(eye[1]<floor){eye[1]=floor;pitch=Math.atan2(eye[1]-targetHeight,horizontal);}
 const sin=Math.sin(pitch),cos=Math.cos(pitch);
 const f=[0,-sin,-cos],u=[0,cos,-sin];
 const near=profile.near,far=profile.far;
 const A=(far+near)/(near-far),B=2*far*near/(near-far);
 const tx=-eye[0],ty=-(u[1]*eye[1]+u[2]*eye[2]),tz=f[1]*eye[1]+f[2]*eye[2];
 const m=view.vp||new Float32Array(16),sx=1/pose.tanX,sy=1/pose.tanY;
 m[0]=sx;m[1]=0;m[2]=0;m[3]=0;
 m[4]=0;m[5]=sy*u[1];m[6]=-A*f[1];m[7]=f[1];
 m[8]=0;m[9]=sy*u[2];m[10]=-A*f[2];m[11]=f[2];
 m[12]=sx*tx;m[13]=sy*ty;m[14]=A*tz+B;m[15]=-tz;
 Object.assign(view,{...pose,pitch,eye,f,u,right:[1,0,0],vp:m,near,far});
 return view;
}

export function originalCameraRay(view,px,py,width,height){
 [width,height]=dimensions(width,height);
 const nx=(px/width*2-1)*view.tanX,ny=(1-py/height*2)*view.tanY;
 return [nx,view.f[1]+view.u[1]*ny,view.f[2]+view.u[2]*ny];
}
