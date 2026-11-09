const NO_MATE=65535;
const f=Math.fround;

export function cityCenterShadowPart(data, descriptor) {
  const {offset,bytes,vertices,faces,edges}=descriptor;
  if (![offset,bytes,vertices,faces,edges].every(Number.isSafeInteger) || offset<0 || offset%4!==0 ||
      vertices<1 || vertices>=NO_MATE || faces<0 || faces>=NO_MATE || edges<0 ||
      bytes!==vertices*12+faces*6+edges*8 || offset+bytes>data.byteLength) {
    throw new RangeError('Invalid City Center shadow buffer');
  }
  const start=data.byteOffset+offset;
  const points=new Float32Array(data.buffer,start,vertices*3);
  const triangles=new Uint16Array(data.buffer,start+vertices*12,faces*3);
  const adjacency=new Uint16Array(data.buffer,start+vertices*12+faces*6,edges*4);
  if (!points.every(Number.isFinite) || triangles.some(index=>index>=vertices)) {
    throw new RangeError('Invalid City Center shadow triangle');
  }
  for (let i=0;i<adjacency.length;i+=4) {
    const a=adjacency[i], b=adjacency[i+1], mate=adjacency[i+2], face=adjacency[i+3];
    if (a>=vertices || b>=vertices || a===b || face>=faces || (mate!==NO_MATE && mate>=faces)) {
      throw new RangeError('Invalid City Center shadow edge');
    }
  }
  return {points,triangles,adjacency};
}

export function cityCenterShadowModels(manifest,data) {
  if (manifest?.format!=='ee2-private-citycenter-topology-v1' || manifest.localOnly!==true ||
      !Array.isArray(manifest.variants) || !(data instanceof Uint8Array)) throw new TypeError('Invalid City Center shadow catalog');
  const models=new Map(), geometries=new Map();
  for (const row of manifest.variants) {
    for (const lod of row.lods) {
      const parts=row.parts.filter(part=>part.lod===lod.lod).map(part=>{
        if(!geometries.has(part.sha256)) geometries.set(part.sha256,cityCenterShadowPart(data,part));
        return {node:part.node,transform:part.transform,geometry:geometries.get(part.sha256)};
      });
      const existing=models.get(lod.url);
      if (existing && (existing.scale!==row.scale || existing.source!==row.source || existing.sha256!==lod.sha256)) {
        throw new TypeError('Conflicting City Center shadow model');
      }
      models.set(lod.url,{parts,scale:row.scale,source:row.source,sha256:lod.sha256});
    }
  }
  return models;
}

function classifyTriangles({points,triangles}, light, store) {
  const facing=new Uint8Array(triangles.length/3);
  for (let i=0;i<triangles.length;i+=3) {
    const a=triangles[i]*3, b=triangles[i+1]*3, c=triangles[i+2]*3;
    const ux=points[b]-points[a], uy=points[b+1]-points[a+1], uz=points[b+2]-points[a+2];
    const vx=store(points[c]-points[a]), vy=store(points[c+1]-points[a+1]), vz=points[c+2]-points[a+2];
    const nx=store(uy*vz-uz*vy), ny=store(uz*vx-ux*vz), nz=ux*vy-uy*vx;
    facing[i/3]=Number(light[2]*nz+light[1]*ny+light[0]*nx<0);
  }
  return facing;
}

export function cityCenterShadowSides(part,light,plane,store=f) {
  const facing=classifyTriangles(part,light,store);
  const inverse=-1/(light[2]*plane[2]+light[1]*plane[1]+light[0]*plane[0]);
  const coefficients=plane.map(value=>store(value*inverse));
  const {points,adjacency}=part;
  const offsets=new Int32Array(points.length/3).fill(-1), positions=[], indices=[];
  function pair(id) {
    if (offsets[id]>=0) return offsets[id];
    const x=points[id*3], y=points[id*3+1], z=points[id*3+2];
    const distance=coefficients[2]*z+coefficients[1]*y+coefficients[0]*x+coefficients[3];
    const offset=positions.length/3;
    positions.push(x,y,z,store(x+light[0]*distance),store(y+store(light[1]*distance)),store(z+store(light[2]*distance)));
    offsets[id]=offset;
    return offset;
  }
  for(let i=0;i<adjacency.length;i+=4) {
    const a=adjacency[i], b=adjacency[i+1], mate=adjacency[i+2], face=adjacency[i+3];
    const lit=facing[face];
    if(lit===(facing[mate]??0)) continue;
    const first=pair(lit?a:b), last=pair(lit?b:a);
    indices.push(first,first+1,last,last,first+1,last+1);
  }
  return {positions,indices,facing};
}

const dot=(matrix,row,[x,y,z])=>matrix[row]*x+matrix[row+1]*y+matrix[row+2]*z;

function inModelSpace({rotation,translation,scale},light) {
  const inverseRotation=[0,3,6,1,4,7,2,5,8].map(index=>rotation[index]);
  const rotate=([x,y,z])=>[
    f(inverseRotation[2]*z+inverseRotation[1]*y+inverseRotation[0]*x),
    f(inverseRotation[5]*z+inverseRotation[3]*x+inverseRotation[4]*y),
    f(inverseRotation[8]*z+inverseRotation[6]*x+inverseRotation[7]*y),
  ];
  const inverseScale=f(1/scale);
  const inverseTranslation=rotate(translation.map(value=>-value)).map(value=>f(value*inverseScale));
  const normal=rotate([0,0,1]);
  const d=normal[2]*inverseTranslation[2]+normal[1]*inverseTranslation[1]+normal[0]*inverseTranslation[0];
  return {light:rotate(light),plane:[...normal,f(-0*inverseScale-d)]};
}

export function cityCenterWorldVolume(model,pose,sourceLight) {
  const [x,y,z,yaw]=pose;
  const c=Math.cos(yaw), s=Math.sin(yaw);
  const root=[c,s,0,-s,c,0,0,0,1];
  const worldLight=[sourceLight[0],-sourceLight[1],sourceLight[2]];
  const positions=[], indices=[];
  for(const {transform,geometry} of model.parts) {
    const rotation=transform.rotation.map((_,i)=>{
      const col=i%3;
      return f(dot(root,i-col,[transform.rotation[col],transform.rotation[col+3],transform.rotation[col+6]]));
    });
    const translation=[x,-y,z].map((value,i)=>f(value+dot(root,3*i,transform.translation)*model.scale));
    const scale=f(model.scale*transform.scale);
    const {light,plane}=inModelSpace({rotation,translation,scale},worldLight);
    const sides=cityCenterShadowSides(geometry,light,plane);
    const base=positions.length/3;
    for(const index of sides.indices) indices.push(index+base);
    for(let i=0;i<sides.positions.length;i+=3) {
      const point=sides.positions.slice(i,i+3);
      const [px,py,pz]=translation.map((value,axis)=>f(value+dot(rotation,3*axis,point)*scale));
      positions.push(px,pz,-py);
    }
  }
  return {positions,indices};
}
