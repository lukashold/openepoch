// Geometry authored by the installed VisualTimerEffect constructor, 006644E0.
// Positions are local to the mode-2 billboard. The caller supplies marker lifetime.
const f32=Math.fround, halfPi=f32(Math.PI/2), step=f32(Math.PI/10), turn=f32(2*Math.PI);
const blue=[.2,.2,.6,.6],gold=[.9,.8,0,1];
const ringSize=21,center=42;
const fixed=new DataView(new ArrayBuffer(8));

function remainingSegments(ratio){
  // 00664CF4 adds 1.5*2^36, then 00ACC7F0 shifts the f64 bits by 16.
  fixed.setFloat64(0,103079215104+ratio*20,true);
  return fixed.getUint32(0,true)>>>16;
}

export function originalExtractionTimerGeometry(remaining,total,{radius=.15,offset=2}={}){
  if(![remaining,total,radius,offset].every(Number.isFinite)||remaining<=0||total<=0||radius<=0)return null;
  const fraction=Math.min(1,f32(remaining)/f32(total)),size=f32(radius),height=f32(offset);
  const positions=new Float32Array(44*3),colors=new Float32Array(44*4),indices=new Uint16Array(40*3);
  const fillPoint=(i,angle)=>{
    positions[i*3]=f32(f32(Math.sin(angle))*size);
    positions[i*3+1]=f32(f32(Math.cos(angle))*size+height);
  };
  const cutoff=ringSize-1-remainingSegments(fraction), angle=halfPi+f32(fraction)*turn;
  for(let i=0;i<ringSize;i++){
    const initial=halfPi-i*step;
    fillPoint(i,initial);
    fillPoint(i+ringSize,i<cutoff?angle:initial);
    colors.set(blue,i*4);colors.set(gold,(i+ringSize)*4);
    if(i<20){
      indices.set([i+1,i,center],i*3);
      indices.set([i+ringSize+1,i+ringSize,center+1],(i+20)*3);
    }
  }
  positions[center*3+1]=positions[(center+1)*3+1]=height;
  colors.set(blue,center*4);colors.set(gold,(center+1)*4);
  return {positions,colors,indices};
}
