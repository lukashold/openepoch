import {CHOP53, NEAREST53} from './source-cloth-arithmetic.js';

// Isolated reconstruction of00619FB0/00619F20. This has no renderer or frame
// clock and never rotates local light by the source attachment/world matrix.
// The original updates geometry positions/RGB and marks NiData dirty|5;
// normals below are diagnostic local temporaries, not a native normal buffer.
export function nativeClothVertexColors({positions,nx,ny},lighting,{rounding='chop53',mode='normal',colors}={}){
  const binary32=value=>Number.isFinite(value)&&Math.fround(value)===value;
  if(!['chop53','nearest53'].includes(rounding)||!['normal','uniform'].includes(mode)||!Number.isInteger(nx)||!Number.isInteger(ny)||nx<2||ny<2||nx>255||ny>255||positions?.length!==nx*ny*3||!positions.every(binary32))throw new RangeError('Source binary32 cloth positions and explicit arithmetic/quality required');
  const {ambient,diffuse,sunDirection}=lighting||{};
  for(const vector of[ambient,diffuse,sunDirection])if(vector?.length!==3||!vector.every(binary32))throw new RangeError('Raw undimmed source RGB and local source ray required');
  const n=nx*ny;
  if(colors!==undefined&&(!(colors instanceof Float32Array)||colors.length!==n*4||!colors.every(Number.isFinite)||colors.buffer===positions.buffer))throw new RangeError('Separate binary32 color target required');
  const rgba=colors??new Float32Array(n*4);
  if(colors===undefined)for(let i=0;i<n;i++)rgba[i*4+3]=1;
  const {add:A,sub:S,mul:M,div:D,sqrt:Q,store32:f}=rounding==='chop53'?CHOP53:NEAREST53;
  const points=Float32Array.from(positions),normals=mode==='normal'?new Float32Array(n*3):null;
  if(mode==='uniform'){
    const rgb=ambient.map((value,i)=>f(A(value,diffuse[i])));
    for(let i=0;i<n;i++)rgba.set(rgb,i*4);
  }else{
    const point=i=>Array.from(points.subarray(i*3,i*3+3));
    const minus=(a,b,stored)=>a.map((value,i)=>stored[i]?f(S(value,b[i])):S(value,b[i]));
    // Source005F9920 stores each cross product component before normalization.
    const cross=(a,b)=>[f(S(M(a[1],b[2]),M(a[2],b[1]))),f(S(M(a[2],b[0]),M(a[0],b[2]))),f(S(M(a[0],b[1]),M(a[1],b[0])))];
    function write(i,a,b,dotOrder){
      let normal=cross(a,b);
      //00615970 and004EC580 share this ASM addition order; do not follow the
      // decompiler's reversed expression or insert an extra float length store.
      const length=Q(A(A(M(normal[0],normal[0]),M(normal[1],normal[1])),M(normal[2],normal[2])));
      if(length>0){const inverse=D(1,length);normal=normal.map(value=>f(M(value,inverse)));}
      normals.set(normal,i*3);
      const dot=Math.abs(A(A(M(normal[dotOrder[0]],sunDirection[dotOrder[0]]),M(normal[dotOrder[1]],sunDirection[dotOrder[1]])),M(normal[dotOrder[2]],sunDirection[dotOrder[2]])));
      for(let channel=0;channel<3;channel++)rgba[i*4+channel]=f(A(M(dot,diffuse[channel]),ambient[channel]));
      // Original code writes no alpha component and does not clamp RGB here.
    }
    const stored=[true,true,true],extended=[false,false,false];
    //0061A087: each top/interior point uses right-self and right-down-left.
    for(let y=0;y<ny-1;y++)for(let x=0;x<nx-1;x++){
      const i=y*nx+x,p=point(i+1),a=minus(p,point(i+nx),stored),b=minus(p,point(i),stored);
      write(i,a,b,[2,0,1]);
    }
    //0061A262: right column uses left-current and left-down-current.
    for(let y=0;y<ny-1;y++){
      const i=y*nx+nx-1,p=point(i-1),a=minus(p,point(i+nx),stored),b=minus(p,point(i),stored);
      write(i,a,b,[2,0,1]);
    }
    //0061A418: compiler leaves all right-self differences extended, and only
    // Y/Z of right-up-left pass through float temporaries before the cross.
    for(let x=0;x<nx-1;x++){
      const i=(ny-1)*nx+x,p=point(i+1),a=minus(p,point(i-nx),[false,true,true]),b=minus(p,point(i),extended);
      write(i,a,b,[2,1,0]);
    }
    //0061A584: last corner stores X/Y (not Z) of left-up-current; left-self
    // stays extended. This distinct store pattern is preserved deliberately.
    {
      const i=n-1,p=point(i-1),a=minus(p,point(i-nx),[true,true,false]),b=minus(p,point(i),extended);
      write(i,a,b,[2,1,0]);
    }
  }
  return {positions:points,colors:rgba,localNormals:normals,sourceFunction:mode==='normal'?'00619FB0':'00619F20',dirtyMask:5,rounding};
}
