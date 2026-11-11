import {originalCameraRaySegment} from './original-camera.js';

const layoutWords = Object.freeze([100,300,128,512,129,513,1]);
const pointEpsilon = 1e-8;

export function activeRectangularLayout(wasm) {
  const info=wasm.active_map_layout;
  if (!info || info(6)===0) return null;
  const words=layoutWords.map((_,field)=>info(field));
  if (words.some((value,index)=>value!==layoutWords[index])) throw Error('Unexpected authenticated rectangular map layout');
  const [width,height,stride,rows,cornerStride,cornerRows]=words;
  return {width,height,stride,rows,cornerStride,cornerRows,cellCount:stride*rows};
}

export function rectangularMinimapSnapshot(wasm, {width,height,stride,rows}) {
  const ptr=wasm.minimap();
  const padded=new Uint8Array(wasm.memory.buffer,ptr,stride*rows*4);
  const compact=new Uint8ClampedArray(width*height*4);
  for (let y=0;y<height;y++) compact.set(padded.subarray(y*stride*4,(y*stride+width)*4),y*width*4);
  return compact;
}

/** W8I1 v2 input has already been authenticated and accepted by the WASM stage. */
export function rectangularCliffFaces(bytes, geometry) {
  const view=new DataView(bytes.buffer,bytes.byteOffset,bytes.byteLength);
  const word=offset=>view.getUint32(offset,true);
  const {width,height,cliffCount}=geometry;
  if (word(0)!==0x31493857 || word(4)!==2 || word(8)!==width || word(12)!==height || word(48)!==cliffCount)
    throw Error('Invalid rectangular registered faces header');
  let offset=84+(width+1)*(height+1)*10+word(20)+width*height*4;
  const faces=new Map();
  for (let i=0;i<cliffCount;i++) {
    const cell=word(offset), count=word(offset+4), words=count*9;
    offset+=8;
    if (cell>=width*height || count<1 || offset+words*4>bytes.length || faces.has(cell))
      throw Error('Invalid rectangular registered face');
    const vertices=new Float32Array(words);
    for (let j=0;j<words;j++) vertices[j]=view.getFloat32(offset+j*4,true);
    faces.set(cell,vertices);
    offset+=words*4;
  }
  return faces;
}

function heightFromCell(geometry,x,y){
  const {width,height,elevations,patterns}=geometry;
  const col=Math.min(width-1,Math.floor(x)), row=Math.min(height-1,Math.floor(y));
  const u=x-col,v=y-row, stride=width+1,at=row*stride+col;
  const a=elevations[at],b=elevations[at+1],c=elevations[at+stride],d=elevations[at+stride+1];
  return patterns[row*width+col]&0xff0000
    ? (u+v<=1?a+u*(b-a)+v*(c-a):d+(1-u)*(c-d)+(1-v)*(b-d))
    : (u<=v?a+u*(d-c)+v*(c-a):a+u*(b-a)+v*(d-b));
}

function triangleRay(origin, direction, vertices, at, first, last) {
  const [ox,oy,oz]=origin,[dx,dy,dz]=direction;
  // Input triangles are native X/Y/height. Camera vectors are X/height/Z.
  const ax=vertices[at],ay=vertices[at+1],az=vertices[at+2];
  const ux=vertices[at+3]-ax,uy=vertices[at+4]-ay,uz=vertices[at+5]-az;
  const vx=vertices[at+6]-ax,vy=vertices[at+7]-ay,vz=vertices[at+8]-az;
  const px=dz*vz-dy*vy,py=dy*vx-dx*vz,pz=dx*vy-dz*vx;
  const det=ux*px+uy*py+uz*pz;
  if (Math.abs(det)<pointEpsilon) return Infinity;
  const tx=ox-ax,ty=oz-ay,tz=oy-az;
  const u=(tx*px+ty*py+tz*pz)/det;
  if (u < -pointEpsilon || u > 1+pointEpsilon) return Infinity;
  const qx=ty*uz-tz*uy,qy=tz*ux-tx*uz,qz=tx*uy-ty*ux;
  const v=(dx*qx+dz*qy+dy*qz)/det;
  if (v < -pointEpsilon || u+v > 1+pointEpsilon) return Infinity;
  const distance=(vx*qx+vy*qy+vz*qz)/det;
  return distance >= first-pointEpsilon && distance <= last+pointEpsilon ? distance : Infinity;
}

export function rectangularGroundAt(world,x,y){
  const {width,height,faces}=world;
  x=Math.min(width,Math.max(0,x)); y=Math.min(height,Math.max(0,y));
  const vertices=x<width&&y<height?faces.get(Math.floor(y)*width+Math.floor(x)):null;
  if (vertices) for (let at=0;at<vertices.length;at+=9) {
    const distance=triangleRay([x,0,y],[0,1,0],vertices,at,-Infinity,Infinity);
    if (Number.isFinite(distance)) return distance;
  }
  return heightFromCell(world,x,y);
}

function horizontalClip(first,last,origin,direction,limit){
  if (direction===0) return origin>=0&&origin<=limit?[first,last]:[Infinity,-Infinity];
  const a=-origin/direction,b=(limit-origin)/direction;
  return [Math.max(first,Math.min(a,b)),Math.min(last,Math.max(a,b))];
}

/** Pick the visible replacement face or the cell's authored diagonal, without scanning the full map. */
export function intersectRectangularGround(world, origin, direction, near, far) {
  const {width,height,elevations,patterns,faces}=world;
  const [ox,oy,oz]=origin,[dx,dy,dz]=direction;
  let [first,end]=horizontalClip(near,far,ox,dx,width);
  [first,end]=horizontalClip(first,end,oz,dz,height);
  if (first>end) return null;
  const sx=Math.sign(dx),sy=Math.sign(dz),stride=width+1;
  const initial=(p,step,limit)=>Math.min(limit-1,Math.max(0,step<0?Math.ceil(p)-1:Math.floor(p)));
  let x=initial(ox+dx*first,sx,width),y=initial(oz+dz*first,sy,height);
  while(x>=0&&x<width&&y>=0&&y<height&&first<=end){
    const tx=dx===0?Infinity:(x+Number(sx>0)-ox)/dx;
    const ty=dz===0?Infinity:(y+Number(sy>0)-oz)/dz;
    const stop=Math.min(tx,ty,end),cell=y*width+x, at=y*stride+x;
    let vertices=faces.get(cell);
    if(!vertices){
      const a=[x,y,elevations[at]],b=[x+1,y,elevations[at+1]],c=[x,y+1,elevations[at+stride]],d=[x+1,y+1,elevations[at+stride+1]];
      vertices=patterns[cell]&0xff0000?[...a,...c,...b,...b,...c,...d]:[...a,...c,...d,...a,...d,...b];
    }
    let nearest=Infinity;
    for(let i=0;i<vertices.length;i+=9)nearest=Math.min(nearest,triangleRay(origin,direction,vertices,i,first,stop));
    if(Number.isFinite(nearest)){
      const px=ox+dx*nearest,py=oz+dz*nearest;
      if(px>=0&&px<width&&py>=0&&py<height)return {x:px,y:py,elevation:oy+dy*nearest,distance:nearest};
    }
    if(stop===end)break;
    if(tx<=ty)x+=sx;
    if(ty<=tx)y+=sy;
    first=stop;
  }
  return null;
}

export function pickRectangularGround(world,view,x,y,width,height){
  if (![x,y,width,height].every(Number.isFinite)||x<0||y<0||x>=width||y>=height) return null;
  const {origin,direction,near,far}=originalCameraRaySegment(view,x,y,width,height);
  return intersectRectangularGround(world,origin,direction,near,far);
}

/** Match the 25-cell groups of the authenticated terrain projection packet. */
export function rectangularBlockBounds(world){
  const {width,height,elevations,faces}=world, span=25, columns=width/span;
  const bounds=[];
  for(let row=0;row<height;row+=span)for(let col=0;col<width;col+=span){
    const edgeX=col+span,edgeY=row+span;
    let low=Infinity,high=-Infinity;
    for(let y=row;y<=edgeY;y++)for(let x=col;x<=edgeX;x++){
      const z=elevations[y*(width+1)+x];
      low=Math.min(low,z);high=Math.max(high,z);
    }
    bounds.push([[col,low,row],[edgeX,high,edgeY]]);
  }
  for(const [cell,vertices] of faces){
    const id=Math.floor((cell%width)/span)+columns*Math.floor(Math.floor(cell/width)/span);
    const [lo,hi]=bounds[id];
    for(let at=2;at<vertices.length;at+=3){lo[1]=Math.min(lo[1],vertices[at]);hi[1]=Math.max(hi[1],vertices[at]);}
  }
  return bounds;
}

export function rectangularVisibleBlocks(bounds,vp){
  const planes=[];
  for(let axis=0;axis<3;axis++)for(const side of [-1,1]){
    planes.push([vp[3]+side*vp[axis],vp[7]+side*vp[axis+4],vp[11]+side*vp[axis+8],vp[15]+side*vp[axis+12]]);
  }
  const visible=[];
  for(const [index,[lo,hi]] of bounds.entries()){
    if(planes.every(([x,y,z,w])=>x*(x<0?lo[0]:hi[0])+y*(y<0?lo[1]:hi[1])+z*(z<0?lo[2]:hi[2])+w>=-.01))visible.push(index);
  }
  return visible;
}
