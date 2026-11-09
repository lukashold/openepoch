import {authoredTerrainMesh} from './authored-view.js';
import {authoredValleyBoundary} from './authored-boundary.js';
import {buildAuthoredTerrainLightmap, SHADOW_BLOCK_SPAN, SHADOW_RESOLUTION} from './authored-terrain-shadow.js';
import {nativeTerrainCoefficients} from './native-terrain-lighting.js';
import {createForestShadowRenderer} from './valley-forest-shadow.js';
import {createRuinVolumeRenderer} from './valley-ruin-shadow.js';
import {createCityCenterShadowRenderer} from './citycenter-shadow-renderer.js';
import {VALLEY_TERRAIN_MASK,valleyMaskMipLevels,valleyMaskShader,valleyTerrainPasses} from './authored-terrain-mask.js?v=valley-mips151';

export const VALLEY_MATERIAL_MIPS = Object.freeze({
  url: 'local-original/valley-terrain/terrain-color-mips.rgba8',
  sha256: 'f6fb9b4e2a4518d33a3293a67b720bd6c08b58e937273aa9fda33f6bf755d63f',
  terrainManifestSha256: '5285919ac3def42c1941767a656b0a89157d285f0f06e20b409953a21fdfc3d4',
});

function valleyMaterialMipLevels(bytes) {
  if (!(bytes instanceof Uint8Array) || bytes.length !== 3495240) return null;
  const levels = [];
  let offset = 0;
  for (let side = 256; side >= 1; side >>= 1) {
    const end = offset + side * side * 4 * 10;
    levels.push({side, pixels: bytes.subarray(offset, end)});
    offset = end;
  }
  return levels;
}

// Keep the corner renderer available when the optional local mask is absent.
const vertex = `#version 300 es
layout(location=0) in vec3 aPosition;
layout(location=1) in vec3 aNormal;
uniform mat4 uVP;
out vec3 vWorld;
out vec3 vNormal;
invariant gl_Position;
void main(){vWorld=aPosition;vNormal=aNormal;gl_Position=uVP*vec4(aPosition,1.0);}
`;
const fragment = `#version 300 es
precision highp float;
precision highp int;
uniform highp sampler2DArray uMaterials;
uniform sampler2D uCorners;
uniform sampler2D uFog;
uniform bool uBoundary;
uniform bool uNativeLighting;
uniform bool uProjectedShadow;
uniform highp sampler2DArray uLightmap;
uniform float uLightmapBlocksX;
uniform vec3 uLightmapDiffuse, uLightmapAmbient;
${valleyMaskShader}
in vec3 vWorld;
in vec3 vNormal;
out vec4 outColor;
vec3 colour(ivec2 at,vec2 uv){
 vec4 corner=texelFetch(uCorners,at,0);
 float layer=floor(corner.r*255.0+0.5);
 return texture(uMaterials,vec3(uv,layer)).rgb*corner.gba*(255.0/128.0);
}
void main(){
 if(uBoundary){outColor=vec4(0.,0.,0.,1.);return;}
 vec2 xy=vWorld.xz;
 vec2 dx=dFdx(xy),dz=dFdy(xy);
 float fog=texture(uFog,xy/256.0).r;
 if(fog<0.015){outColor=vec4(0.,0.,0.,1.);return;}
 ivec2 cell=clamp(ivec2(floor(xy)),ivec2(0),textureSize(uCorners,0)-2);
 vec2 pos=clamp(xy-vec2(cell),0.,1.);
 vec3 land;
 if(uNativeMask)land=maskedTerrain(cell,pos,dx,dz);
 else{
  vec2 uv=xy/4.0;
  vec3 a=colour(cell,uv),d=colour(cell+ivec2(1,1),uv);
  if(pos.y>=pos.x){land=a*(1.0-pos.y)+colour(cell+ivec2(0,1),uv)*(pos.y-pos.x)+d*pos.x;}
  else{land=a*(1.0-pos.x)+colour(cell+ivec2(1,0),uv)*(pos.x-pos.y)+d*pos.y;}
 }
 if(uNativeLighting){
  vec2 block=floor(xy/${SHADOW_BLOCK_SPAN}.0);
  vec2 blockUV=(xy-block*${SHADOW_BLOCK_SPAN}.0)*((1.-1./${SHADOW_RESOLUTION}.0)/${SHADOW_BLOCK_SPAN}.0)+vec2(.5/${SHADOW_RESOLUTION}.0);
  float mask=uProjectedShadow?0.0:texture(uLightmap,vec3(blockUV,block.x+block.y*uLightmapBlocksX)).r;
  vec3 stage0=clamp(2.*mask*uLightmapDiffuse,0.,1.);
  vec3 stage1=clamp(stage0+uLightmapAmbient,0.,1.);
  land=clamp(2.*land*stage1,0.,1.);
 }else{
  vec3 sun=normalize(vec3(.42,.82,.39));
  land*=vec3(.55,.57,.59)+vec3(.53,.50,.43)*max(dot(normalize(vNormal),sun),0.0);
 }
 float reveal=smoothstep(.02,.45,fog)*mix(.5,1.0,smoothstep(.55,.95,fog));
 outColor=vec4(land*reveal,1.0);
}
`;

export function valleyTerrainCornerTexture(geometry, manifest) {
  if (manifest?.format !== 'ee2-private-valley-terrain-v1' || manifest.localOnly !== true || manifest.month !== 6 ||
      manifest.sourceArchiveSha256 !== geometry.sourceArchiveSha256 || manifest.dimension?.join(',') !== '115,71' ||
      !Array.isArray(manifest.bindings) || manifest.bindings.length !== 10) throw new Error('Valley material pack differs from the active map');
  const ids = new Map();
  for (const [layer, row] of manifest.bindings.entries()) {
    if (!Number.isInteger(row.materialId) || ids.has(row.materialId) ||
        !/^local-original\/terrain-native\/[a-z0-9_]+\.png$/.test(row.url) || !/^[0-9a-f]{64}$/.test(row.sha256)) {
      throw new Error('Invalid Valley material binding');
    }
    ids.set(row.materialId, layer);
  }
  const counts = new Uint32Array(ids.size);
  const cornerTexture = new Uint8Array(geometry.materials.length * 4);
  for (const [index, material] of geometry.materials.entries()) {
    const layer = ids.get(material), tint = geometry.tints[index];
    if (layer === undefined) throw new Error('Unbound authored material ID');
    counts[layer]++;
    cornerTexture.set([layer, (tint >>> 16) & 255, (tint >>> 8) & 255, tint & 255], index * 4);
  }
  if (manifest.bindings.some((row, layer) => row.count !== counts[layer])) throw new Error('Authored material counts differ');
  return cornerTexture;
}

function program(gl, vertexSource, fragmentSource) {
  const result = gl.createProgram();
  for (const [kind, source] of [[gl.VERTEX_SHADER, vertexSource], [gl.FRAGMENT_SHADER, fragmentSource]]) {
    const shader = gl.createShader(kind);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
    gl.attachShader(result, shader);
    gl.deleteShader(shader);
  }
  gl.linkProgram(result);
  if (!gl.getProgramParameter(result, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(result));
  return result;
}

const hex = bytes => Array.from(new Uint8Array(bytes), n => n.toString(16).padStart(2, '0')).join('');

export async function loadAuthoredValleyTerrain(gl, geometry, manifest, readTexture, lighting = null, forestShadow = null,
  maskMips = null, materialMips = null, ruinShadow = null, cityShadow = null) {
  const corners = valleyTerrainCornerTexture(geometry, manifest);
  const {vertices, indices} = authoredTerrainMesh(geometry);
  const boundary = authoredValleyBoundary(geometry);
  const materialLevels = valleyMaterialMipLevels(materialMips);
  const images = materialLevels ? [] : await Promise.all(manifest.bindings.map(async row => {
    const image = await readTexture(row.url);
    if (!image?.bytes || hex(await crypto.subtle.digest('SHA-256', image.bytes)) !== row.sha256 ||
        image.bitmap?.width !== 256 || image.bitmap?.height !== 256) throw new Error('Valley terrain texture differs: ' + row.url);
    return image.bitmap;
  }));
  const mipLevels = valleyMaskMipLevels(maskMips);
  const mask = mipLevels ? null : await readTexture(VALLEY_TERRAIN_MASK.url).catch(()=>null);
  const bitmap = mask?.bitmap;
  const masked = Boolean(mipLevels || (bitmap?.width === 256 && bitmap?.height === 256 && mask?.bytes &&
    hex(await crypto.subtle.digest('SHA-256',mask.bytes)) === VALLEY_TERRAIN_MASK.sha256));
  const passes = masked ? valleyTerrainPasses(geometry, manifest.bindings) : new Uint8Array(4);
  const shader = program(gl,vertex,fragment);
  const uniform = name => gl.getUniformLocation(shader, name);
  const uniforms = [uniform('uVP'), uniform('uMaterials'), uniform('uCorners'), uniform('uFog'), uniform('uLightmap'),
    null, // Forest coverage reserves texture unit 4.
    uniform('uMaterialPasses'), uniform('uTerrainMask')];
  const uNativeMask = uniform('uNativeMask'), uMaskMipChain = uniform('uMaskMipChain');
  const uNativeLighting = uniform('uNativeLighting'), uBlocksX = uniform('uLightmapBlocksX');
  const uProjectedShadow = uniform('uProjectedShadow');
  const uDiffuse = uniform('uLightmapDiffuse'), uAmbient = uniform('uLightmapAmbient');
  const uBoundary = uniform('uBoundary');
  const vao = gl.createVertexArray();
  gl.bindVertexArray(vao);
  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER, vertices, gl.STATIC_DRAW);
  for (let attribute = 0; attribute < 2; attribute++) {
    gl.enableVertexAttribArray(attribute);
    gl.vertexAttribPointer(attribute, 3, gl.FLOAT, false, 24, 12 * attribute);
  }
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, indices, gl.STATIC_DRAW);

  const boundaryVao = gl.createVertexArray();
  gl.bindVertexArray(boundaryVao);
  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER, boundary.vertices, gl.STATIC_DRAW);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 12, 0);
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, gl.createBuffer());
  gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, boundary.indices, gl.STATIC_DRAW);

  gl.activeTexture(gl.TEXTURE0);
  const layerTexture = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D_ARRAY, layerTexture);
  const layers = manifest.bindings.length;
  gl.texStorage3D(gl.TEXTURE_2D_ARRAY, 9, gl.RGBA8, 256, 256, layers);
  if (materialLevels) {
    for (const [level, {side, pixels}] of materialLevels.entries()) {
      gl.texSubImage3D(gl.TEXTURE_2D_ARRAY, level, 0, 0, 0, side, side, layers, gl.RGBA, gl.UNSIGNED_BYTE, pixels);
    }
  } else {
    for (const [layer, image] of images.entries()) {
      gl.texSubImage3D(gl.TEXTURE_2D_ARRAY, 0, 0, 0, layer, 256, 256, 1, gl.RGBA, gl.UNSIGNED_BYTE, image);
      image.close();
    }
  }
  gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
  gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  if (!materialLevels) gl.generateMipmap(gl.TEXTURE_2D_ARRAY);
  for (const axis of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T]) gl.texParameteri(gl.TEXTURE_2D_ARRAY, axis, gl.REPEAT);
  const cornerTexture = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0 + 1);
  gl.bindTexture(gl.TEXTURE_2D, cornerTexture);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, geometry.width + 1, geometry.height + 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, corners);
  for (const axis of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T]) gl.texParameteri(gl.TEXTURE_2D, axis, gl.CLAMP_TO_EDGE);
  for (const mode of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, mode, gl.NEAREST);

  const passTexture = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0 + 5);
  gl.bindTexture(gl.TEXTURE_2D, passTexture);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8UI, masked ? geometry.width : 1, masked ? geometry.height : 1,
    0, gl.RGBA_INTEGER, gl.UNSIGNED_BYTE, passes);
  for (const mode of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, mode, gl.NEAREST);
  const maskTexture = masked ? gl.createTexture() : cornerTexture;
  if (masked) {
    gl.activeTexture(gl.TEXTURE0 + 6);
    gl.bindTexture(gl.TEXTURE_2D, maskTexture);
    if (mipLevels) {
      gl.texStorage2D(gl.TEXTURE_2D,mipLevels.length,gl.R8,256,256);
      const alignment = gl.getParameter(gl.UNPACK_ALIGNMENT);
      gl.pixelStorei(gl.UNPACK_ALIGNMENT,1);
      try {
        for (const [level,{side,pixels}] of mipLevels.entries())
          gl.texSubImage2D(gl.TEXTURE_2D,level,0,0,side,side,gl.RED,gl.UNSIGNED_BYTE,pixels);
      } finally {
        gl.pixelStorei(gl.UNPACK_ALIGNMENT,alignment);
      }
    } else {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE, bitmap);
    }
    for (const axis of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T]) gl.texParameteri(gl.TEXTURE_2D, axis, gl.REPEAT);
    for (const mode of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, mode, gl.LINEAR);
  }
  bitmap?.close();

  const lightmap = lighting?.native && lighting.climate === 'Temperate' && lighting.sourceTerrainDirection
    ? buildAuthoredTerrainLightmap(geometry, lighting.sourceTerrainDirection) : null;
  const coefficients = lightmap && nativeTerrainCoefficients(lighting);
  const lightmapTexture = lightmap ? gl.createTexture() : layerTexture;
  if (lightmap) {
    gl.activeTexture(gl.TEXTURE0 + 3);
    gl.bindTexture(gl.TEXTURE_2D_ARRAY, lightmapTexture);
    gl.texImage3D(gl.TEXTURE_2D_ARRAY, 0, gl.R8, SHADOW_RESOLUTION, SHADOW_RESOLUTION, lightmap.layers,
      0, gl.RED, gl.UNSIGNED_BYTE, lightmap.data);
    for (const axis of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T]) gl.texParameteri(gl.TEXTURE_2D_ARRAY, axis, gl.CLAMP_TO_EDGE);
    for (const mode of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D_ARRAY, mode, gl.LINEAR);
  }
  const diffuse = coefficients?.diffuseBytes.map(value => value / 255);
  const ambient = coefficients?.ambientBytes.map(value => value / 255);
  const projectForest = lightmap && forestShadow && gl.getContextAttributes().stencil
    ? createForestShadowRenderer(gl,geometry,indices,forestShadow,lighting.sourceTerrainDirection,program) : null;
  const projectRuins = lightmap && ruinShadow && gl.getContextAttributes().stencil
    ? createRuinVolumeRenderer(gl,ruinShadow,program) : null;
  const projectCities = lightmap && cityShadow && gl.getContextAttributes().stencil
    ? createCityCenterShadowRenderer(gl,cityShadow,lighting.sourceTerrainDirection,program) : null;
  const anisotropy = mipLevels && gl.getExtension('EXT_texture_filter_anisotropic');
  const maxAnisotropy = anisotropy ? Math.min(4,gl.getParameter(anisotropy.MAX_TEXTURE_MAX_ANISOTROPY_EXT)) : 1;
  let samplerHigh;
  return {
    indexCount: indices.length,
    draw(vp, fogTexture, highShadows = false, highSampling = false, ruins = [], cities = []) {
      const drawnCities=highShadows && projectCities && cities.length ? projectCities.prepare(cities) : null;
      gl.useProgram(shader);
      gl.uniform1i(uBoundary, 0);
      gl.uniform1i(uNativeMask, masked ? 1 : 0);
      gl.uniform1i(uMaskMipChain, mipLevels ? 1 : 0);
      gl.uniform1i(uNativeLighting, lightmap ? 1 : 0);
      gl.uniform1i(uProjectedShadow, 0);
      if (lightmap) {
        gl.uniform1f(uBlocksX, lightmap.blocksX);
        gl.uniform3fv(uDiffuse, diffuse);
        gl.uniform3fv(uAmbient, ambient);
      }
      gl.uniformMatrix4fv(uniforms[0], false, vp);
      for (const [unit, target, texture] of [[0, gl.TEXTURE_2D_ARRAY, layerTexture], [1, gl.TEXTURE_2D, cornerTexture],
        [2, gl.TEXTURE_2D, fogTexture], [3, gl.TEXTURE_2D_ARRAY, lightmapTexture],
        [5, gl.TEXTURE_2D, passTexture], [6, gl.TEXTURE_2D, maskTexture]]) {
        gl.activeTexture(gl.TEXTURE0 + unit);
        gl.bindTexture(target, texture);
        gl.uniform1i(uniforms[unit + 1], unit);
      }
      if ((materialLevels || mipLevels) && samplerHigh !== highSampling) {
        samplerHigh = highSampling;
        const minFilter = highSampling ? gl.LINEAR_MIPMAP_LINEAR : gl.LINEAR_MIPMAP_NEAREST;
        gl.activeTexture(gl.TEXTURE0);
        gl.texParameteri(gl.TEXTURE_2D_ARRAY,gl.TEXTURE_MIN_FILTER,minFilter);
        gl.activeTexture(gl.TEXTURE0 + 6);
        if (mipLevels) {
          gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,minFilter);
          // The native high branch requests anisotropic MIPFILTER. WebGL represents it with trilinear mips.
          if (anisotropy) gl.texParameterf(gl.TEXTURE_2D,anisotropy.TEXTURE_MAX_ANISOTROPY_EXT,highSampling ? maxAnisotropy : 1);
        }
      }
      gl.bindVertexArray(vao);
      gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);
      const drawRuins = projectRuins && ruins.length;
      const drawVolumes = drawRuins || drawnCities?.size;
      if (highShadows && (drawVolumes || projectForest)) {
        const depth = gl.getParameter(gl.DEPTH_FUNC);
        if (drawVolumes) {
          gl.depthMask(false);
          gl.depthFunc(gl.LEQUAL);
          gl.colorMask(false,false,false,false);
          gl.enable(gl.STENCIL_TEST);
          gl.stencilMask(0x3f);
          gl.stencilFunc(gl.ALWAYS,0,255);
          if(drawRuins)projectRuins(vp,ruins);
          if(drawnCities?.size)projectCities.draw(vp);
        }
        gl.depthMask(false);
        gl.depthFunc(drawVolumes ? gl.LEQUAL : gl.EQUAL);
        gl.colorMask(false,false,false,false);
        gl.enable(gl.STENCIL_TEST);
        gl.stencilMask(0x3f);
        gl.stencilFunc(gl.ALWAYS,0,0x3f);
        gl.stencilOp(gl.KEEP,gl.KEEP,gl.INCR_WRAP);
        if (projectForest) projectForest(vp);
        gl.colorMask(true,true,true,true);
        gl.stencilMask(0);
        gl.stencilOp(gl.KEEP,gl.KEEP,gl.KEEP);
        gl.useProgram(shader);
        gl.bindVertexArray(vao);
        if (drawVolumes) {
          gl.stencilFunc(gl.EQUAL,0,0x3f);
          gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);
        }
        gl.stencilFunc(gl.NOTEQUAL,0,0x3f);
        gl.uniform1i(uProjectedShadow, 1);
        gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);
        gl.disable(gl.STENCIL_TEST);
        gl.stencilMask(0x3f);
        gl.clear(gl.STENCIL_BUFFER_BIT);
        gl.stencilMask(255);
        gl.depthFunc(depth);
        gl.depthMask(true);
      }
      gl.uniform1i(uBoundary, 1);
      gl.bindVertexArray(boundaryVao);
      let first = 0;
      for (const length of boundary.stripLengths) {
        gl.drawElements(gl.TRIANGLE_STRIP, length, gl.UNSIGNED_SHORT, first * Uint16Array.BYTES_PER_ELEMENT);
        first += length;
      }
      return drawnCities;
    },
  };
}
