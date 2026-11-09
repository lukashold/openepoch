import {projectedTerrainTiles, uprightForestProjector} from './valley-forest-projector.js';

const sourceManifest = {
  url: 'local-original/valley-shadows/manifest.json',
  sha256: '43f6e3e4b2bebb2c0706151194f4dac441ff26e4c322cbd1c5dc7575696d2464',
};
const SIDE = 64, LAYERS = 4, STRIDE = 6, UNIT = 4;
const hex = buffer => Array.from(new Uint8Array(buffer), byte => byte.toString(16).padStart(2,'0')).join('');

export async function loadForestShadowAssets(checked, forest, forestBytes) {
  const manifest = JSON.parse(new TextDecoder().decode(await checked(sourceManifest)));
  if (manifest.format !== 'ee2-private-valley-upright-shadows-v1' || manifest.localOnly !== true ||
      manifest.sourceArchiveSha256 !== forest.sourceArchiveSha256 || manifest.profiles.length !== forest.models.length ||
      manifest.forestSha256 !== hex(await crypto.subtle.digest('SHA-256',forestBytes)) ||
      manifest.atlas.dimensions.join(',') !== [SIDE,SIDE,LAYERS].join(',')) throw new Error('Valley shadow pack differs from the active forest');
  const pixels = await checked(manifest.atlas);
  if (pixels.length !== SIDE * SIDE * LAYERS * 4) throw new Error('Invalid Valley shadow texture size');
  const float = new DataView(new ArrayBuffer(4));
  const decode = word => {
    float.setUint32(0,word,true);
    return float.getFloat32(0,true);
  };
  const profiles = manifest.profiles.map((profile,index) => {
    if (profile.model !== forest.models[index].id || !Number.isInteger(profile.layer) ||
        profile.layer < 0 || profile.layer >= LAYERS || profile.minBits.length !== 3 || profile.maxBits.length !== 3) {
      throw new Error('Invalid Valley shadow profile');
    }
    return {layer:profile.layer,bounds:[...profile.minBits,...profile.maxBits].map(decode),
      radius:decode(profile.radiusBits),height:decode(profile.heightBits)};
  });
  return {forest,profiles,pixels};
}

const vertex = `#version 300 es
layout(location=0) in vec3 aPosition;
layout(location=1) in vec3 aCoverage;
uniform mat4 uVP;
out vec3 vCoverage;
invariant gl_Position;
void main(){vCoverage=aCoverage;gl_Position=uVP*vec4(aPosition,1.0);}
`;
const fragment = `#version 300 es
precision highp float;
uniform highp sampler2DArray uForestCoverage;
in vec3 vCoverage;
out vec4 outColor;
void main(){
 if(texture(uForestCoverage,vCoverage).a<128.0/255.0)discard;
 outColor=vec4(0.0);
}
`;

function forestShadowGeometry(geometry, indices, assets, direction) {
  const {width,height,elevations} = geometry;
  const stride = width + 1;
  let tileCount = 0;
  const entries = assets.forest.rows.map(([qx,qy,,model]) => {
    const {bounds,radius,height: heightScale,layer} = assets.profiles[model];
    const projection = uprightForestProjector(qx/128,qy/128,direction,bounds,radius,heightScale);
    const tiles = projectedTerrainTiles(width,height,projection);
    tileCount += tiles.length;
    return {projection,layer,tiles};
  });
  const vertices = new Float32Array(tileCount * 6 * STRIDE);
  let offset = 0;
  const ends = new Uint32Array(entries.length + 1);
  for (const [row,{projection:{u,v},layer,tiles}] of entries.entries()) {
    for (const cell of tiles) {
      for (let i = 0; i < 6; i++) {
        const corner = indices[cell*6+i], x = corner % stride, y = Math.floor(corner / stride);
        vertices.set([x,elevations[corner],y,u[0]*x+u[1]*y-u[3],v[0]*x+v[1]*y-v[3],layer],offset);
        offset += STRIDE;
      }
    }
    ends[row + 1] = offset / STRIDE;
  }
  return {vertices,ends};
}

export function forestShadowVertices(geometry, indices, assets, direction) {
  return forestShadowGeometry(geometry,indices,assets,direction).vertices;
}

export function createForestShadowRenderer(gl, geometry, indices, assets, direction, compile) {
  const {vertices,ends} = forestShadowGeometry(geometry,indices,assets,direction);
  const program = compile(gl,vertex,fragment);
  const vp = gl.getUniformLocation(program,'uVP'), coverage = gl.getUniformLocation(program,'uForestCoverage');
  const vao = gl.createVertexArray();
  gl.bindVertexArray(vao);
  gl.bindBuffer(gl.ARRAY_BUFFER,gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER,vertices,gl.STATIC_DRAW);
  for (let i = 0; i < 2; i++) {
    gl.enableVertexAttribArray(i);
    gl.vertexAttribPointer(i,3,gl.FLOAT,false,STRIDE*4,i*12);
  }
  const texture = gl.createTexture();
  const target = gl.TEXTURE_2D_ARRAY;
  gl.activeTexture(gl.TEXTURE0 + UNIT);
  gl.bindTexture(target,texture);
  gl.texImage3D(target,0,gl.RGBA8,SIDE,SIDE,LAYERS,0,gl.RGBA,gl.UNSIGNED_BYTE,assets.pixels);
  for (const axis of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T]) gl.texParameteri(target,axis,gl.CLAMP_TO_EDGE);
  for (const mode of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER]) gl.texParameteri(target,mode,gl.LINEAR);
  const anisotropic = gl.getExtension('EXT_texture_filter_anisotropic');
  if (anisotropic) gl.texParameterf(target,anisotropic.TEXTURE_MAX_ANISOTROPY_EXT,
    Math.min(4,gl.getParameter(anisotropic.MAX_TEXTURE_MAX_ANISOTROPY_EXT)));
  let elements = null, compressed = null, active = null, count = 0;
  const drawActive = phases => {
    if (!(phases instanceof Uint8Array) || phases.length + 1 !== ends.length) throw new TypeError('Invalid forest shadow phases');
    if (!elements) {
      elements = gl.createBuffer();
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,elements);
      active = new Uint32Array(ends.at(-1));
    }
    if (!compressed?.every((present,row)=>present === Number(phases[row] === 0))) {
      compressed = Uint8Array.from(phases,phase=>Number(phase === 0));
      count = 0;
      for (const [row,present] of compressed.entries()) {
        if (!present) continue;
        for (let index=ends[row]; index<ends[row+1]; index++) active[count++] = index;
      }
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,active.subarray(0,count),gl.DYNAMIC_DRAW);
    }
    if (count) gl.drawElements(gl.TRIANGLES,count,gl.UNSIGNED_INT,0);
  };
  return (matrix, phases = null) => {
    gl.useProgram(program);
    gl.uniformMatrix4fv(vp,false,matrix);
    gl.uniform1i(coverage,UNIT);
    gl.activeTexture(gl.TEXTURE0 + UNIT);
    gl.bindTexture(target,texture);
    gl.bindVertexArray(vao);
    if (phases instanceof Uint8Array) drawActive(phases);
    else gl.drawArrays(gl.TRIANGLES,0,vertices.length / STRIDE);
  };
}
