import {authoredTerrainMesh} from './authored-view.js';
import {authoredValleyBoundary} from './authored-boundary.js';

// Compact corner palette, exact source geometry and original terrain pixels.
// This browser shader uses triangle interpolation for material edges. The
// original mask-staging and per-tile UV pattern are separate source work.
const vertex = `#version 300 es
layout(location=0) in vec3 aPosition;
layout(location=1) in vec3 aNormal;
uniform mat4 uVP;
out vec3 vWorld;
out vec3 vNormal;
void main(){vWorld=aPosition;vNormal=aNormal;gl_Position=uVP*vec4(aPosition,1.0);}
`;
const fragment = `#version 300 es
precision highp float;
uniform highp sampler2DArray uMaterials;
uniform sampler2D uCorners;
uniform sampler2D uFog;
uniform bool uBoundary;
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
 float fog=texture(uFog,xy/256.0).r;
 if(fog<0.015){outColor=vec4(0.,0.,0.,1.);return;}
 ivec2 cell=clamp(ivec2(floor(xy)),ivec2(0),textureSize(uCorners,0)-2);
 vec2 pos=clamp(xy-vec2(cell),0.,1.),uv=xy/4.0;
 vec3 a=colour(cell,uv),d=colour(cell+ivec2(1,1),uv),land;
 if(pos.y>=pos.x){land=a*(1.0-pos.y)+colour(cell+ivec2(0,1),uv)*(pos.y-pos.x)+d*pos.x;}
 else{land=a*(1.0-pos.x)+colour(cell+ivec2(1,0),uv)*(pos.x-pos.y)+d*pos.y;}
 vec3 sun=normalize(vec3(.42,.82,.39));
 vec3 light=vec3(.55,.57,.59)+vec3(.53,.50,.43)*max(dot(normalize(vNormal),sun),0.0);
 float reveal=smoothstep(.02,.45,fog)*mix(.5,1.0,smoothstep(.55,.95,fog));
 outColor=vec4(land*light*reveal,1.0);
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

function program(gl) {
  const result = gl.createProgram();
  for (const [kind, source] of [[gl.VERTEX_SHADER, vertex], [gl.FRAGMENT_SHADER, fragment]]) {
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

export async function loadAuthoredValleyTerrain(gl, geometry, manifest, readTexture) {
  const corners = valleyTerrainCornerTexture(geometry, manifest);
  const {vertices, indices} = authoredTerrainMesh(geometry);
  const boundary = authoredValleyBoundary(geometry);
  const images = await Promise.all(manifest.bindings.map(async row => {
    const image = await readTexture(row.url);
    if (!image?.bytes || hex(await crypto.subtle.digest('SHA-256', image.bytes)) !== row.sha256 ||
        image.bitmap?.width !== 256 || image.bitmap?.height !== 256) throw new Error('Valley terrain texture differs: ' + row.url);
    return image.bitmap;
  }));
  const shader = program(gl);
  const uniform = name => gl.getUniformLocation(shader, name);
  const uniforms = [uniform('uVP'), uniform('uMaterials'), uniform('uCorners'), uniform('uFog')];
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
  gl.texStorage3D(gl.TEXTURE_2D_ARRAY, 9, gl.RGBA8, 256, 256, images.length);
  for (const [layer, image] of images.entries()) {
    gl.texSubImage3D(gl.TEXTURE_2D_ARRAY, 0, 0, 0, layer, 256, 256, 1, gl.RGBA, gl.UNSIGNED_BYTE, image);
    image.close();
  }
  gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
  gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.generateMipmap(gl.TEXTURE_2D_ARRAY);
  const cornerTexture = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0 + 1);
  gl.bindTexture(gl.TEXTURE_2D, cornerTexture);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, geometry.width + 1, geometry.height + 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, corners);
  for (const axis of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T]) gl.texParameteri(gl.TEXTURE_2D, axis, gl.CLAMP_TO_EDGE);
  for (const mode of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, mode, gl.NEAREST);

  return {
    indexCount: indices.length,
    draw(vp, fogTexture) {
      gl.useProgram(shader);
      gl.uniform1i(uBoundary, 0);
      gl.uniformMatrix4fv(uniforms[0], false, vp);
      for (const [unit, target, texture] of [[0, gl.TEXTURE_2D_ARRAY, layerTexture], [1, gl.TEXTURE_2D, cornerTexture], [2, gl.TEXTURE_2D, fogTexture]]) {
        gl.activeTexture(gl.TEXTURE0 + unit);
        gl.bindTexture(target, texture);
        gl.uniform1i(uniforms[unit + 1], unit);
      }
      gl.bindVertexArray(vao);
      gl.drawElements(gl.TRIANGLES, indices.length, gl.UNSIGNED_INT, 0);
      gl.uniform1i(uBoundary, 1);
      gl.bindVertexArray(boundaryVao);
      let first = 0;
      for (const length of boundary.stripLengths) {
        gl.drawElements(gl.TRIANGLE_STRIP, length, gl.UNSIGNED_SHORT, first * Uint16Array.BYTES_PER_ELEMENT);
        first += length;
      }
    },
  };
}
