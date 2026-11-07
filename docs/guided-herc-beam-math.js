// Pure model for the authored middle section of Beam_Herc01. Input points
// and its basis use original XYZ axes. The caller supplies the live tag and
// target transforms from the same presentation view.
const f=Math.fround;
export const delay=f(.35), life=f(.4), death=f(.4), speed=f(200);
export const fadeStart=f(delay+life), middleEnd=f(delay+life+death);

const cross=([x,y,z],[a,b,c])=>[f(y*c-z*b),f(z*a-x*c),f(x*b-y*a)];
const dot=(a,b)=>a[0]*b[0]+a[1]*b[1]+a[2]*b[2];
function unit(point){
 const length=Math.hypot(...point);
 return length>0?point.map(value=>f(value/length)):point;
}

export function middleBasis(tagRotation,from,to){
 // 006370A0 first applies the inverse tag matrix. Its three columns
 // are deliberately not a general orthonormal look-at frame.
 const [x,y,z]=tagRotation;
 const cofactors=[cross(y,z),cross(z,x),cross(x,y)], determinant=dot(x,cofactors[0]);
 if(Math.abs(determinant)<1e-8)return null;
 const direction=unit(to.map((value,i)=>f(value-from[i])));
 const local=cofactors.map(row=>f(dot(row,direction)/determinant));
 const up=[0,0,1],side=unit(cross(up,local));
 const toWorld=([a,b,c])=>x.map((value,i)=>f(value*a+y[i]*b+z[i]*c));
 return [side.map(value=>-value),local.map(value=>-value),cross(side,up)].map(toWorld);
}

export function advanceMiddle(previous,delta,distance){
 const age=f(previous.age+delta);
 let reached=previous.reached;
 if(age>=delay)reached=Math.min(distance,f(reached+speed*delta));
 if(age<delay||age>=middleEnd||!(reached>0))return {age,reached,draw:false};
 let from=1,to=1;
 if(age>=fadeStart){
  to=f(1-(age-fadeStart)/death);
  from=f(Math.max(0,to-.5));
 }
 // 00636520 interpolates the tip alpha against the entire target distance.
 return {age,reached,draw:true,nearAlpha:from,farAlpha:f(from+(to-from)*reached/distance)};
}

export function authoredMiddleVertices(mesh,frame,point,basis,scale){
 if(!frame.draw)return new Float32Array();
 const authoredSpan=Math.max(...mesh.positions.map(p=>p[1]))-Math.min(...mesh.positions.map(p=>p[1]));
 const yRatio=f((frame.reached/scale)/authoredSpan),at=new Float32Array(mesh.positions.length*6);
 for(let i=0;i<mesh.positions.length;i++){
  const [x,y,z]=mesh.positions[i],localY=f(y*yRatio),offset=i*6;
  const local=[x+mesh.translation[0],localY+mesh.translation[1],z+mesh.translation[2]];
  for(let j=0;j<3;j++)at[offset+j]=point[j]+scale*(basis[0][j]*local[0]+basis[1][j]*local[1]+basis[2][j]*local[2]);
  at[offset+3]=mesh.uv[i][0]*yRatio;
  at[offset+4]=mesh.uv[i][1];
  at[offset+5]=frame.nearAlpha-(frame.farAlpha-frame.nearAlpha)*localY/(frame.reached/scale);
 }
 return at;
}

// Native laser_middle unlit vertex colour and fixed-function MODULATE4X.
// Source coordinates are X,Y,Z, while the shared view uses X,Z,Y.
export const MIDDLE_VERTEX=`#version 300 es
precision highp float;
layout(location=0) in vec3 aSource;
layout(location=1) in vec2 aUV;
layout(location=2) in float aAlpha;
uniform mat4 uVP;
out vec2 vUV;
out float vAlpha;
void main(){
 gl_Position=uVP*vec4(aSource.x,aSource.z,aSource.y,1.);
 vUV=aUV;
 vAlpha=aAlpha;
}`;

export const MIDDLE_FRAGMENT=`#version 300 es
precision highp float;
uniform sampler2D uBeam;
in vec2 vUV;
in float vAlpha;
out vec4 outColor;
void main(){
 vec4 texel=texture(uBeam,vUV);
 float alpha=texel.a*vAlpha;
 if(alpha<=0.)discard;
 outColor=vec4(clamp(texel.rgb*4.,0.,1.),alpha);
}`;
