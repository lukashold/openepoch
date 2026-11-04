import {CHOP53, NEAREST53} from './source-cloth-arithmetic.js';

// Direct source constants from00A9D410/00659E40 and PE float bytes.
export const CLOTH_SOURCE_CONSTANTS = Object.freeze({
  seed:12345, multiplier:1664525, increment:1013904223,
  gravity:-32.17399978637695, drag:0.9900000095367432, spring:300,
  springRecordDamping:2, totalMass:100, third:0.3333333432674408,
  randomScale:0.000030518509447574615, maxDelta:0.05000000074505806,
});

// 00659730 intentionally retains the old mask below1/9. It does not select8.
export function nativeSailDamageMask(previous, fraction) {
  if(!Number.isInteger(previous)||previous<0||previous>8||!Number.isFinite(fraction))throw new RangeError('Invalid source damage state');
  for(const [limit,mask]of [[0.8888888955116272,0],[0.7777777910232544,1],[0.6666666865348816,2],[0.5555555820465088,3],[0.4444444477558136,4],[0.3333333432674408,5],[0.2222222238779068,6],[0.1111111119389534,7]])if(fraction>=limit)return mask;
  return previous;
}

// 00658D20, before the source's attachment-inverse-rotation multiplication.
// Caller supplies original Z-up XY delta explicitly; no own-map axis/clock
// conversion or weather-wind field is silently inferred by this helper.
export function nativeMotionWorldForces(deltaXY,{moving=false,windDamping=0,suppressWind=false,rounding='chop53'}={}){
  if(!['chop53','nearest53'].includes(rounding)||deltaXY?.length!==2||!deltaXY.every(Number.isFinite)||!Number.isFinite(windDamping))throw new RangeError('Invalid native motion force inputs');
  const math=rounding==='chop53'?CHOP53:NEAREST53,{store32:f,add:A,mul:M,div:D,sqrt:Q}=math;
  let x=f(deltaXY[0]),y=f(deltaXY[1]);
  const length=Q(A(M(y,y),M(x,x)));
  if(length>0){const inverse=D(1,length);x=f(M(x,inverse));y=f(M(y,inverse));}
  const coefficient=moving?M(f(windDamping)||1,.5):0.07500000298023224;
  const scale=M(coefficient,600);
  const low=suppressWind?[0,0,0]:[f(M(x,scale)),f(M(y,scale)),0];
  return {gravity:[0,0,1],windMin:low,windMax:low.map(value=>f(M(value,4))),normalizedDelta:[x,y]};
}

// 008E6970 reads only row-major matrix floats0..8. Although008EE030 also
// produces inverse scale/translation, the source force path never reads them.
export function nativeInverseTagRotation(rotation){
  if(rotation?.length!==9||!rotation.every(value=>Number.isFinite(value)&&Math.fround(value)===value))throw new RangeError('Source binary32 tag rotation required');
  // 008E6BA0 is a bit-preserving transpose, not a general matrix inverse.
  return [rotation[0],rotation[3],rotation[6],rotation[1],rotation[4],rotation[7],rotation[2],rotation[5],rotation[8]];
}

export function nativeRotateForce(inverseRotation,vector,{rounding='chop53'}={}){
  if(!['chop53','nearest53'].includes(rounding)||inverseRotation?.length!==9||vector?.length!==3||![...inverseRotation,...vector].every(Number.isFinite))throw new RangeError('Invalid source force rotation');
  const {store32:f,add:A,mul:M}=rounding==='chop53'?CHOP53:NEAREST53;
  const m=inverseRotation.map(f),[x,y,z]=vector.map(f);
  return [f(A(A(M(m[2],z),M(m[1],y)),M(x,m[0]))),f(A(A(M(m[5],z),M(m[3],x)),M(m[4],y))),f(A(A(M(m[8],z),M(m[6],x)),M(m[7],y)))];
}

export function nativeLocalForces(worldForces,inverseRotation,options){
  return Object.fromEntries(['gravity','windMin','windMax'].map(key=>[key,nativeRotateForce(inverseRotation,worldForces[key],options)]));
}

export function createNativeCloth({positions, nx, ny, pinned}, {rounding='chop53', seed=CLOTH_SOURCE_CONSTANTS.seed}={}) {
  if(!['chop53','nearest53'].includes(rounding))throw new RangeError('Explicit source arithmetic mode required');
  if(!Number.isInteger(nx)||!Number.isInteger(ny)||nx<2||ny<2||nx>255||ny>255||nx*ny>65535||positions?.length!==nx*ny*3||pinned?.length!==nx*ny)throw new RangeError('Invalid source grid');
  if(!positions.every(value=>Number.isFinite(value)&&Math.fround(value)===value&&Math.abs(value)<1e10)||!Number.isInteger(seed)||seed<0||seed>0xffffffff)throw new RangeError('Invalid source cloth state; local binary32 points required');
  const math=rounding==='chop53'?CHOP53:NEAREST53;
  const f=math.store32,A=math.add,S=math.sub,M=math.mul,D=math.div,Q=math.sqrt;
  const count=nx*ny, source=CLOTH_SOURCE_CONSTANTS;
  const point=new Float32Array(positions),velocity=new Float32Array(count*3),force=new Float32Array(count*3),mass=new Float32Array(count),inverseMass=new Float32Array(count),pins=Uint8Array.from(pinned,value=>value?1:0);
  const springs=[];
  const length=(x,y,z)=>Q(A(A(M(x,x),M(y,y)),M(z,z))); //00615970 ASM order
  const triangleMass=f(D(source.totalMass,2*(nx-1)*(ny-1)));
  for(let y=0;y<ny;y++)for(let x=0;x<nx;x++){
    const i=y*nx+x;
    // Native00A9DA10 compares y==NY/x==NX, not NY−1/NX−1. Preserve that
    // boundary behavior; replacing it with a modern uniform mass grid differs.
    const coefficient=y===0&&x===0?1:y===ny&&x===0||y===0&&x===nx?2:y===ny&&x===nx?1:(y===0||y===ny)&&x!==0&&x!==nx?3:6;
    mass[i]=f(M(M(coefficient,triangleMass),source.third));inverseMass[i]=f(D(1,mass[i]));
    const append=j=>{
      const dx=f(S(point[i*3],point[j*3])),dy=f(S(point[i*3+1],point[j*3+1])),dz=f(S(point[i*3+2],point[j*3+2]));
      springs.push({a:i,b:j,rest:f(length(dx,dy,dz)),stiffness:source.spring,damping:source.springRecordDamping});
    };
    if(x<nx-1)append(i+1);
    if(y<ny-1)append(i+nx);
    if(x<nx-1&&y<ny-1)append(i+nx+1);
    if(x>0&&y<ny-1)append(i+nx-1);
  }
  let state=seed>>>0,steps=0;

  function substep(dt,gravity,wind){
    for(const spring of springs){
      const a=spring.a*3,b=spring.b*3;
      let dx=f(S(point[a],point[b])),dy=f(S(point[a+1],point[b+1])),dz=f(S(point[a+2],point[b+2]));
      const distance=length(dx,dy,dz);
      if(distance===0)continue;
      const inverse=D(1,distance);dx=f(M(dx,inverse));dy=f(M(dy,inverse));dz=f(M(dz,inverse));
      const magnitude=-M(S(distance,spring.rest),spring.stiffness),fy=f(M(dy,magnitude)),fz=f(M(dz,magnitude));
      // Source X product remains in the FPU until accumulation; Y/Z are stored
      // as float temporaries first. These are deliberate distinct store points.
      force[a]=f(A(M(dx,magnitude),force[a]));force[a+1]=f(A(fy,force[a+1]));force[a+2]=f(A(fz,force[a+2]));
      force[b]=f(S(force[b],M(dx,magnitude)));force[b+1]=f(S(force[b+1],fy));force[b+2]=f(S(force[b+2],fz));
    }
    for(let i=0;i<count;i++){
      const at=i*3;
      if(!pins[i]){
        const gravityMass=M(source.gravity,mass[i]);
        for(let axis=0;axis<3;axis++)force[at+axis]=f(A(force[at+axis],f(M(gravityMass,gravity[axis]))));
        let x=-velocity[at],y=-velocity[at+1],z=-velocity[at+2];
        const speed=length(x,y,z);
        if(speed>0){const inverse=D(1,speed);x=f(M(x,inverse));y=f(M(y,inverse));z=f(M(z,inverse));}
        const drag=M(M(speed,source.drag),speed);
        for(const [axis,normal]of [[0,x],[1,y],[2,z]])force[at+axis]=f(A(force[at+axis],f(M(normal,drag))));
        for(let axis=0;axis<3;axis++){
          force[at+axis]=f(A(force[at+axis],wind[axis]));
          const acceleration=f(M(inverseMass[i],force[at+axis]));
          const change=f(M(acceleration,dt));
          velocity[at+axis]=f(A(change,velocity[at+axis]));
          const distance=f(M(velocity[at+axis],dt));
          point[at+axis]=f(A(distance,point[at+axis]));
        }
      }
      force[at]=force[at+1]=force[at+2]=0;
    }
  }

  return {
    positions:point,velocity,mass,inverseMass,pins,springs,rounding,
    get seed(){return state;},get steps(){return steps;},
    releasePins(){pins.fill(0);}, //00A9DFB0, not a freeze
    step(delta,{gravity,windMin,windMax}){
      if(!Number.isFinite(delta)||delta<0)throw new RangeError('Invalid cloth delta');
      for(const v of [gravity,windMin,windMax])if(v?.length!==3||!v.every(value=>Number.isFinite(value)&&Math.abs(value)<1e10))throw new RangeError('Explicit source force vectors required');
      delta=f(delta);if(delta>source.maxDelta)delta=source.maxDelta;
      const half=f(M(delta,.5));
      //00A9DFE0 advances the same RNG three times before reading its word+2
      // through three aliased pointers. All components share the final sample.
      for(let i=0;i<3;i++)state=(Math.imul(state,source.multiplier)+source.increment)>>>0;
      const sample=(state>>>16)&0x7fff,wind=new Float32Array(3);
      for(let axis=0;axis<3;axis++){
        const low=f(M(f(windMin[axis]),.5)),high=f(M(f(windMax[axis]),.5));
        wind[axis]=f(A(M(M(S(high,low),sample),source.randomScale),low));
      }
      const g=gravity.map(f);substep(half,g,wind);substep(half,g,wind);steps++;
      return {delta,halfDelta:half,randomWord:sample,wind,seed:state};
    },
    snapshot(){return {format:'source-cloth-kernel-state-v1',nx,ny,rounding,seed:state,steps,positions:Array.from(point),velocity:Array.from(velocity),pins:Array.from(pins)};},
  };
}
