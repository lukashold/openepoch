import {nativeTerrainCoefficients} from './native-terrain-lighting.js';

const sourceArchive='8f0f6028cda68dcaa395fffda9973f83a7f24ea22f2a12361cafa1fc392bcec4';
const elevationHash='dd113c221305b035db41078aff9d6c04431389d4ca2aba54025d94fa5258c18a';
const atlasHash='935e36e72ee47c8e7e75b74c7728f2d3a2c30ebeeb4e6f82e7a3c7256a8e410b';
const media='local-original/death-valley2-terrain/lightmaps.r8';
const periodNames=['Morning','Afternoon','Evening','Night'];
const directions=[
  [3210749600,1053875757,3196351466],
  [3206119441,1046553147,3208969715],
  [3208175646,1058672665,3199113927],
  [3205844893,1060217067,3201842774],
];
const side=128,layers=48,blocksX=4,atlasBytes=side*side*layers;
const hash=async bytes=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),
  value=>value.toString(16).padStart(2,'0')).join('');

async function readLocal(url) {
  const response=await fetch(new URL(url,import.meta.url),{cache:'no-store'});
  if(!response.ok)throw Error('Lokale Klippenbeleuchtung fehlt.');
  return new Uint8Array(await response.arrayBuffer());
}

export async function loadAuthoredCliffLightmap(gl,geometry,read=readLocal) {
  if(geometry?.sourceArchiveSha256!==sourceArchive||geometry.width!==100||geometry.height!==300||
     !(geometry.elevations instanceof Float32Array)||geometry.elevations.length!==101*301||!globalThis.crypto?.subtle)return null;
  if(await hash(new Uint8Array(geometry.elevations.buffer,geometry.elevations.byteOffset,geometry.elevations.byteLength))!==elevationHash){
    throw Error('Die Geländehöhen stimmen nicht mit der Klippenbeleuchtung überein.');
  }
  const bytes=await read(media);
  if(!(bytes instanceof Uint8Array)||bytes.length!==atlasBytes*periodNames.length||await hash(bytes)!==atlasHash){
    throw Error('Ungültige lokale Klippenbeleuchtung.');
  }
  const textures=new Map();
  let disposed=false;

  function textureFor(index) {
    if(textures.has(index))return textures.get(index);
    const target=gl.TEXTURE_2D_ARRAY;
    const previous=gl.getParameter(gl.TEXTURE_BINDING_2D_ARRAY);
    const fields=[gl.UNPACK_ALIGNMENT,gl.UNPACK_ROW_LENGTH,gl.UNPACK_IMAGE_HEIGHT,gl.UNPACK_SKIP_PIXELS,gl.UNPACK_SKIP_ROWS,gl.UNPACK_SKIP_IMAGES];
    const previousUnpack=fields.map(field=>gl.getParameter(field));
    const previousBuffer=gl.getParameter(gl.PIXEL_UNPACK_BUFFER_BINDING);
    const texture=gl.createTexture();
    if(!texture)throw Error('Klippenbeleuchtung kann nicht reserviert werden.');
    try{
      gl.bindTexture(target,texture);
      gl.bindBuffer(gl.PIXEL_UNPACK_BUFFER,null);
      fields.forEach((field,slot)=>gl.pixelStorei(field,slot===0?1:0));
      gl.texImage3D(target,0,gl.R8,side,side,layers,0,gl.RED,gl.UNSIGNED_BYTE,bytes.subarray(index*atlasBytes,(index+1)*atlasBytes));
      for(const axis of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(target,axis,gl.CLAMP_TO_EDGE);
      for(const filter of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(target,filter,gl.LINEAR);
      textures.set(index,texture);
      return texture;
    }catch(error){
      gl.deleteTexture(texture);
      throw error;
    }finally{
      gl.bindBuffer(gl.PIXEL_UNPACK_BUFFER,previousBuffer);
      fields.forEach((field,slot)=>gl.pixelStorei(field,previousUnpack[slot]));
      gl.bindTexture(target,previous);
    }
  }

  function select(lighting) {
    if(disposed||!lighting?.native||lighting.climate!=='Temperate')return null;
    const index=periodNames.indexOf(lighting.period);
    if(index<0||lighting.sourceTerrainDirection?.length!==3)return null;
    const direction=new Uint32Array(Float32Array.from(lighting.sourceTerrainDirection).buffer);
    if(!direction.every((word,axis)=>word===directions[index][axis]))return null;
    const coefficients=nativeTerrainCoefficients(lighting);
    return {texture:textureFor(index),blocksX,diffuse:coefficients.diffuseBytes.map(value=>value/255),
      ambient:coefficients.ambientBytes.map(value=>value/255)};
  }

  function dispose() {
    for(const texture of textures.values())gl.deleteTexture(texture);
    textures.clear();
    disposed=true;
  }

  return {select,dispose};
}
