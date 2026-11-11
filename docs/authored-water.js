import {stockFrustumFromWebGL} from './camera-frustum.js';
import {buildOriginalWaterBodies} from './original-water-bodies.js';
import {sourceCameraProjection} from './dv2-reflection-scene.js';
import {createOriginalWaterBodyUV} from './native-water-body-uv.js';

const mapSha = '747e308eb8b10edb3333531989c577e15e87b2d424716c075c0c3bad8a980899';
const inputSha = '4f399e0bb21752444edb545923aa2e22558bb4aabcca27f40bb193773f5a33d9';
const palette = Object.freeze([
  0,0,0,3,7,0,0,2,3,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,6,5,0,0,0,0,0,0,
]);
const dimensions = Object.freeze([90,90]);
const fullViewport = Object.freeze([0,1,1,0]);
const f32 = Math.fround;
const forward = ([x,height,y]) => [f32(x),f32(y),f32(height)];
const hex = bytes => Array.from(bytes, n=>n.toString(16).padStart(2,'0')).join('');

/** Island90's pinned GOG archive or the independently reviewed World8 packet. */
export async function readIsland90WaterInput(bytes, {subtle = globalThis.crypto?.subtle} = {}) {
  if (!(bytes instanceof Uint8Array) || !subtle || bytes.length > 16 * 1024 * 1024) {
    throw new TypeError('Island90 requires bounded binary input and SHA-256 support');
  }
  const copy = new Uint8Array(bytes);
  const digest = hex(new Uint8Array(await subtle.digest('SHA-256',copy)));
  const [width,height] = dimensions;
  const n = (width+1)*(height+1);
  const view = new DataView(copy.buffer);
  let offset, movePalette;
  if (digest === mapSha) {
    offset = 245; // Terr chunk at 217, corner dimensions at 237/241.
    if (view.getUint32(237,true) !== width+1 || view.getUint32(241,true) !== height+1) {
      throw new RangeError('Invalid authenticated Island90 terrain dimensions');
    }
    movePalette = Uint8Array.from(palette);
  } else if (digest === inputSha) {
    offset = 84;
    if (view.getUint32(0,true) !== 0x31493857 || view.getUint32(4,true) !== 2 ||
        view.getUint32(8,true) !== width || view.getUint32(12,true) !== height ||
        view.getUint32(20,true) !== palette.length || hex(copy.subarray(52,84)) !== mapSha) {
      throw new RangeError('Invalid authenticated Island90 World8 packet');
    }
    movePalette = copy.slice(offset+10*n,offset+10*n+palette.length);
    if (!movePalette.every((value,i)=>value === palette[i])) throw new Error('Island90 movement palette differs');
  } else {
    throw new Error('Island90 source digest mismatch');
  }
  const elevations = Float32Array.from({length:n},(_,i)=>view.getFloat32(offset+4*i,true));
  const materials = Uint16Array.from({length:n},(_,i)=>view.getUint16(offset+4*n+2*i,true));
  return {width,height,elevations,materials,movePalette,sourceArchiveSha256:mapSha,inputSha256:digest};
}

/** Forward authored X/Y corners with source water Z=0 and no simulation padding. */
export function prepareAuthoredWater({width,height,elevations,materials,movePalette}, {rounding='chop'} = {}) {
  const sampleWidth = width+1, sampleHeight = height+1, count = sampleWidth*sampleHeight;
  if (!Number.isInteger(width) || !Number.isInteger(height) || width<1 || height<1 || width>512 || height>512 ||
      width*height>65536 ||
      !(elevations instanceof Float32Array) || elevations.length !== count || !elevations.every(Number.isFinite) ||
      !(materials instanceof Uint16Array) || materials.length !== count ||
      !(movePalette instanceof Uint8Array) || !movePalette.length || movePalette.some(kind=>kind>7) ||
      materials.some(index=>index>=movePalette.length)) throw new RangeError('Invalid authored water corner fields');
  const movement = Uint8Array.from(materials,index=>movePalette[index]);
  const positionUV = new Float32Array(4*count);
  const uv = createOriginalWaterBodyUV({rounding});
  for (let y=0,i=0;y<sampleHeight;y++) for (let x=0;x<sampleWidth;x++,i+=4) {
    const [u,v] = uv(x,y);
    positionUV[i] = x;
    positionUV[i+1] = y;
    positionUV[i+2] = u;
    positionUV[i+3] = v;
  }
  const bodies = buildOriginalWaterBodies(elevations,sampleWidth,sampleHeight);
  return {width,height,sampleWidth,sampleHeight,elevations,movement,bodies,positionUV};
}

export function authoredWaterIndices(bodies, sampleWidth) {
  const indices = new Uint32Array(bodies.reduce((count,body)=>count+(body.indices?.length??0),0));
  let at = 0;
  for (const body of bodies) for (const index of body.indices??[]) {
    const x = body.x+index%body.width;
    const y = body.y+Math.floor(index/body.width);
    indices[at++] = y*sampleWidth+x;
  }
  return indices;
}

/** The same authored world axes apply to ordinary and overview cameras. */
export function authoredDirectReflectionCamera(view, viewport=fullViewport) {
  const {eye,f,u,right,tanX,tanY,near,far} = view;
  return {eye:forward(eye),forward:forward(f),up:forward(u),right:forward(right),viewport,
    frustum:{left:f32(-tanX),right:f32(tanX),top:f32(tanY),bottom:f32(-tanY),
      near:f32(near),far:f32(far),orthographic:false}};
}

export function authoredReflectionProjection(camera) {
  const view = sourceCameraProjection(camera,0);
  // Zero map height selects the canonical negative-Y basis; switch only Z.
  for (const name of ['eye','forward','up','right']) view[name][2] = -view[name][2];
  for (let i=8;i<12;i++) view.vp[i] = -view.vp[i];
  return view;
}

export function authoredWaterFrustum(view, height) {
  return stockFrustumFromWebGL(view.vp,height,true);
}
