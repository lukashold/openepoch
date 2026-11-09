import {stockFrustumFromWebGL} from './camera-frustum.js?v=native-water-presentation231';
import {perspectiveGridRay} from './reflection-grid.js?v=native-water-presentation231';
import {cloneReflectionCamera,createReflectionPacketCache,prepareReflectionViews,reflectionRoute,
  refreshReflectionBodies} from './original-reflection-policy.js?v=native-water-presentation231';
import {buildOriginalWaterBodies} from './original-water-bodies.js?v=native-water-presentation231';

const f32 = Math.fround;
const fullViewport = Object.freeze([0,1,1,0]);
const roleByTarget = ['main','pip'];
const dot = ([ax,ay,az],[bx,by,bz]) => (ax*bx+ay*by)+az*bz;
const sourceVector = ([x,y,z]) => [f32(x),f32(-z),f32(y)];
const worldVector = ([x,y,z]) => [x,z,-y];

export function sourceWaterBodies(heights,size) {
  const side=size+1,source=new Float32Array(heights.length);
  for (let row=0;row<side;row++) source.set(heights.subarray(row*side,(row+1)*side),(size-row)*side);
  return buildOriginalWaterBodies(source,side,side);
}

export function sourceWaterIndices(bodies,size) {
  const indices=new Uint32Array(bodies.reduce((count,body)=>count+(body.indices?.length??0),0));
  let at=0;
  for (const body of bodies) for (const index of body.indices??[]) {
    const x=body.x+index%body.width,y=body.y+Math.floor(index/body.width);
    indices[at++]=(size-y)*(size+1)+x;
  }
  return indices;
}

export function directReflectionCamera(view, mapHeight, viewport = fullViewport) {
  const {eye,f,u,right,tanX,tanY,near,far} = view;
  return {
    eye:[f32(eye[0]),f32(mapHeight-eye[2]),f32(eye[1])],
    forward:sourceVector(f),up:sourceVector(u),right:sourceVector(right),viewport,
    frustum:{left:f32(-tanX),right:f32(tanX),top:f32(tanY),bottom:f32(-tanY),near:f32(near),far:f32(far),orthographic:false},
  };
}

export function reflectionGridCamera(camera) {
  const {left,right,top,bottom,near,far,orthographic} = camera.frustum;
  return {...camera,orthographic,frustum:[left,right,top,bottom,near,far]};
}

/** Translate the original six camera fields into a WebGL clip matrix. */
export function sourceCameraProjection(camera,mapHeight) {
  const eye = [camera.eye[0],camera.eye[2],mapHeight-camera.eye[1]];
  const [forward,up,right] = [camera.forward,camera.up,camera.right].map(worldVector);
  const frustum = camera.frustum;
  const width = frustum.right-frustum.left, height = frustum.top-frustum.bottom;
  const depth = frustum.far-frustum.near;
  const halfX = (frustum.right+frustum.left)/width, halfY = (frustum.top+frustum.bottom)/height;
  const combine = (a,scale,b,shift) => a.map((value,i)=>value*scale-b[i]*shift);
  const row = (vector,shift = 0) => [...vector,shift-dot(vector,eye)];
  const ortho = frustum.orthographic;
  const x = ortho ? row(right.map(v=>2*v/width),-halfX) : row(combine(right,2/width,forward,halfX));
  const y = ortho ? row(up.map(v=>2*v/height),-halfY) : row(combine(up,2/height,forward,halfY));
  const z = ortho ? row(forward.map(v=>2*v/depth),-(frustum.far+frustum.near)/depth)
    : row(forward.map(v=>v*(frustum.far+frustum.near)/depth),-2*frustum.far*frustum.near/depth);
  const w = ortho ? [0,0,0,1] : row(forward);
  const vp = Float32Array.from(x.flatMap((value,index)=>[value,y[index],z[index],w[index]]));
  return {eye,vp,forward,up,right};
}

// The ordinary pass refreshes its list after preparation and both reflection
// captures. A reflected draw never refreshes a direct camera's body list.
export function createReflectionSceneSchedule({mapHeight,targetSupported,deviceAvailable=true,clipStateAllowed=true}) {
  let bodies = [], previous = null, frame = null, screenWidth, screenHeight, refreshClone;
  const buildCloneCache = () => createReflectionPacketCache(camera=>cloneReflectionCamera(camera,{
    screenWidth,screenHeight,
    screenRay:(initial,x,y)=>perspectiveGridRay(reflectionGridCamera(initial),screenWidth,screenHeight,x,y),
  }));

  function reset(nextBodies) {
    bodies = nextBodies;
    previous = null;
    frame = null;
    refreshClone = buildCloneCache();
  }

  function prepare({main,pip,qualitySetting,screenWidth:width,screenHeight:height}) {
    if (!main || !refreshClone) throw Error('Reflection schedule requires a world and main camera');
    screenWidth = width;
    screenHeight = height;
    const direct = [main,pip].filter(Boolean).map(({view,viewport},flags)=>({
      flags, camera:directReflectionCamera(view,mapHeight,viewport), view,
    }));
    const state = {
      bodyCount:bodies.length,mainVisibleCount:previous?.mainVisibleCount??0,pipVisibleCount:previous?.pipVisibleCount??0,
      deviceAvailable,clipStateAllowed,targetSupported,qualitySetting,
    };
    const ordered = prepareReflectionViews(direct,state,refreshClone);
    const captures = ordered.slice(0,ordered.length-direct.length).map(packet=>{
      const source = direct[packet.flags&1];
      const {target} = reflectionRoute(source,direct[0].camera);
      const role = roleByTarget[target.id];
      return {role,id:role==='main'?'reflection':'pip-reflection',target,packet,
        view:sourceCameraProjection(packet.camera,mapHeight),clipPlane:[0,1,0,0]};
    });
    const preparation = [...captures.map(({id,view})=>({id,vp:view.vp})),
      ...direct.map((source,index)=>({id:roleByTarget[index],vp:source.view.vp}))];
    frame = {direct,captures,preparation,state,screenWidth,screenHeight};
    return frame;
  }

  function refreshDirect(current,role) {
    if (current !== frame) throw Error('Reflection draw used a stale frame');
    const source = frame.direct[role === 'main' ? 0 : 1];
    if (!source) throw Error('Missing direct reflection camera');
    previous = refreshReflectionBodies({bodies,camera:source.camera,canonicalCamera:frame.direct[0].camera,
      planes:stockFrustumFromWebGL(source.view.vp,mapHeight),previous});
    return role === 'main' ? previous.mainBodies : previous.pipBodies;
  }

  const resetCameraLists=()=>reset(bodies);
  return {reset,resetCameraLists,prepare,refreshDirect};
}
