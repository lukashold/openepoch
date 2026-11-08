// Source lightmap operations over OpenEpoch's own heightfield. Native DEM,
// cliff queries and projected shadow emitters are separate unclosed inputs.
const f=Math.fround,RES=128,SPAN=25;
export {nativeTerrainCoefficients} from './native-terrain-lighting.js';
export const LIGHTMAP_LAYOUT=Object.freeze({resolution:RES,blockSpan:SPAN,textureUnit:14});
const finite3=v=>v&&v.length===3&&Array.from(v).every(Number.isFinite);
export function validateTerrainLighting(value){
 if(!value||!finite3(value.direction)||!finite3(value.sun)||!finite3(value.ambient))throw new RangeError('Native terrain lighting requires finite direction, sun and ambient triples');
 if(Math.abs(Math.hypot(...value.direction)-1)>1e-4)throw new RangeError('Native terrain light direction must be source-normalized');
 if([...value.sun,...value.ambient].some(v=>v<0)||!value.terrain||!['diffuseScale','ambientScale','shadowScale'].every(k=>Number.isFinite(value.terrain[k])&&value.terrain[k]>=0))throw new RangeError('Invalid native terrain source coefficients');
 return {direction:Array.from(value.direction),sun:Array.from(value.sun),ambient:Array.from(value.ambient),terrain:{...value.terrain}};
}
export function buildOwnTerrainNormals(heights,size){
 const stride=size+1,normals=new Float32Array(heights.length*3),h=(x,z)=>heights[Math.max(0,Math.min(size,z))*stride+Math.max(0,Math.min(size,x))];
 for(let z=0;z<=size;z++)for(let x=0;x<=size;x++){
  const i=z*stride+x,n=i*3,nx=h(x-1,z)-h(x+1,z),nz=h(x,z-1)-h(x,z+1),length=Math.hypot(nx,2,nz);
  if(heights[i]<f(.05)){normals[n+1]=1;continue;}
  normals[n]=nx/length;normals[n+1]=2/length;normals[n+2]=nz/length;
 }
 return normals;
}
export function ownTriangleHeight(heights,size,x,z,sourceDiagonal=false){
 x=Math.max(0,Math.min(size-1e-6,x));z=Math.max(0,Math.min(size-1e-6,z));
 const ix=Math.floor(x),iz=Math.floor(z),u=x-ix,v=z-iz,i=iz*(size+1)+ix;
 const a=heights[i],b=heights[i+1],c=heights[i+size+1],d=heights[i+size+2];
 if(sourceDiagonal)return u<=v?a+(d-c)*u+(c-a)*v:a+(b-a)*u+(d-b)*v;
 return u+v<=1?a+(b-a)*u+(c-a)*v:d+(c-d)*(1-u)+(b-d)*(1-v);
}
export function ownTerrainOccluded(heights,size,x,z,direction,maximum,sourceDiagonal=false){
 if(x<0||z<0||x>=size||z>=size)return false;
 const [dx,dy,dz]=direction,y=Math.max(0,ownTriangleHeight(heights,size,x,z,sourceDiagonal))+f(.01);
 if(Math.abs(dx)+Math.abs(dz)<1e-14)return dy<0;
 let cx=Math.floor(x),cz=Math.floor(z),entered=0,steps=0;
 const sx=dx>0?1:-1,sz=dz>0?1:-1,dtx=dx?1/Math.abs(dx):Infinity,dtz=dz?1/Math.abs(dz):Infinity;
 let tx=dx?(cx+(dx>0?1:0)-x)/dx:Infinity,tz=dz?(cz+(dz>0?1:0)-z)/dz:Infinity;
 while(cx>=0&&cz>=0&&cx<size&&cz<size&&entered<=1024){
  if(dy>0&&y+dy*entered>maximum+.00001)return false;
  const end=Math.min(1024,tx,tz),i=cz*(size+1)+cx,a=heights[i],b=heights[i+1],c=heights[i+size+1],d=heights[i+size+2];
  for(let upper=0;upper<2;upper++){
   const px=sourceDiagonal?(upper?b-a:d-c):(upper?d-c:b-a);
   const pz=upper?d-b:c-a,origin=upper&&!sourceDiagonal?b+c-d:a,denominator=dy-px*dx-pz*dz;
   const t=Math.abs(denominator)>1e-12?(origin+px*(x-cx)+pz*(z-cz)-y)/denominator:Infinity;
   const u=x+dx*t-cx,v=z+dz*t-cz;
   const half=sourceDiagonal?(upper?u>=v-1e-7:u<=v+1e-7):(upper?u+v>=1-1e-7:u+v<=1+1e-7);
   if(t>=entered-1e-7&&t<=end+1e-7&&t>0&&half&&u>=-1e-7&&v>=-1e-7&&u<=1+1e-7&&v<=1+1e-7)return true;
  }
  const crossX=tx<=tz,crossZ=tz<=tx;entered=Math.min(tx,tz);
  if(crossX){cx+=sx;tx+=dtx;}if(crossZ){cz+=sz;tz+=dtz;}
  if(++steps>2*size+4)throw new Error('Own terrain DDA exceeded map traversal bound');
 }
 return false;
}
export function sampleNativeNormal(normals,size,x,z,out=[0,0,0]){
 x=Math.max(0,Math.min(size,x));z=Math.max(0,Math.min(size,z));
 const ix=Math.min(Math.floor(x),size-1),iz=Math.min(Math.floor(z),size-1),u=x-ix,v=z-iz,a=(iz*(size+1)+ix)*3,b=a+3,c=a+(size+1)*3,d=c+3;
 for(let k=0;k<3;k++)out[k]=u<=v?normals[a+k]*(1-v)+normals[c+k]*(v-u)+normals[d+k]*u:normals[a+k]*(1-u)+normals[b+k]*(u-v)+normals[d+k]*v;
 const length=Math.sqrt(out[0]*out[0]+out[1]*out[1]+out[2]*out[2]);
 if(length>0)for(let k=0;k<3;k++)out[k]=f(out[k]/length);
 return out;
}
export function buildNativeTerrainLightmap({heights,size=256,normals=null,direction,sourceDiagonal=false}){
 if(!Number.isInteger(size)||size<1||size>1024||!heights||heights.length!==(size+1)**2||!Array.from(heights).every(Number.isFinite))throw new RangeError('Invalid native lightmap heightfield');
 if(!finite3(direction)||Math.abs(Math.hypot(...direction)-1)>1e-4)throw new RangeError('Invalid native lightmap direction');
 if(normals&&(!ArrayBuffer.isView(normals)||normals.length!==heights.length*3||!Array.from(normals).every(Number.isFinite)))throw new RangeError('Invalid native lightmap normals');
 normals=normals?new Float32Array(normals):buildOwnTerrainNormals(heights,size);
 //006F8400 overrides the lightmap copy's low-water normals; source geometry
 //and normals passed by the caller are never modified.
 for(let i=0;i<heights.length;i++)if(heights[i]<f(.05))normals.set([0,1,0],i*3);
 const blocks=Math.ceil(size/SPAN),data=new Uint8Array(blocks*blocks*RES*RES),positions=new Float32Array(RES),step=f(SPAN/(RES-1));
 for(let i=1;i<RES;i++)positions[i]=f(positions[i-1]+step);
 let maximum=-Infinity,occludedSamples=0;for(const h of heights)maximum=Math.max(maximum,h);
 const bits=new Uint8Array(RES*RES),normal=[0,0,0];
 for(let bz=0;bz<blocks;bz++)for(let bx=0;bx<blocks;bx++){
  for(let z=0;z<RES;z++)for(let x=0;x<RES;x++){
   const blocked=ownTerrainOccluded(heights,size,f(positions[x]+f(bx*SPAN)),f(positions[z]+f(bz*SPAN)),direction,maximum,sourceDiagonal);
   bits[z*RES+x]=blocked?1:0;if(blocked)occludedSamples++;
  }
  const offset=(bz*blocks+bx)*RES*RES;
  for(let z=0;z<RES;z++)for(let x=0;x<RES;x++){
   let weighted=0;
   for(let j=-1;j<=1;j++)for(let i=-1;i<=1;i++)weighted+=bits[Math.max(0,Math.min(RES-1,z+j))*RES+Math.max(0,Math.min(RES-1,x+i))]*(i===0?2:1)*(j===0?2:1);
   const visibility=Math.trunc((1-weighted/16)*255);
   sampleNativeNormal(normals,size,f(positions[x]+f(bx*SPAN)),f(positions[z]+f(bz*SPAN)),normal);
   data[offset+z*RES+x]=Math.max(0,Math.trunc((normal[0]*direction[0]+normal[1]*direction[1]+normal[2]*direction[2])*visibility));
  }
 }
 return {data,resolution:RES,blockSpan:SPAN,blocks,layers:blocks*blocks,occludedSamples};
}
