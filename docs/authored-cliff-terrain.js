const prefix = 'local-original/death-valley2-terrain/';
const archive = '8f0f6028cda68dcaa395fffda9973f83a7f24ea22f2a12361cafa1fc392bcec4';
export const AUTHORED_CLIFF_TERRAIN = Object.freeze({
  url: prefix + 'manifest.json',
  sha256: 'ab9c401743518ac875b8694e812d8d7f9adaee054c3e9492bb2cad8fe01287aa',
  sourceArchiveSha256: archive,
});

const width = 100, height = 300, blocksX = 4, blockCount = 48;
const f32 = Math.fround;

export function admitsAuthoredCliffTerrain(geometry) {
  return geometry?.sourceArchiveSha256 === archive && geometry.width === width && geometry.height === height &&
    geometry.elevations instanceof Float32Array && geometry.elevations.length === (width + 1) * (height + 1);
}

async function readLocal(url) {
  const response = await fetch(new URL(url, import.meta.url), { cache: 'no-store' });
  if (!response.ok) throw new Error('Private cliff terrain asset unavailable');
  return new Uint8Array(await response.arrayBuffer());
}

async function digest(bytes) {
  const words = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  return Array.from(words, word => word.toString(16).padStart(2, '0')).join('');
}

async function checked(read, { url, sha256, bytes }) {
  if (!/^local-original\/[a-z0-9_.\/-]+$/.test(url) || url.includes('..') || !/^[a-f0-9]{64}$/.test(sha256)) {
    throw new Error('Invalid private cliff terrain asset');
  }
  const body = await read(url);
  if (!(body instanceof Uint8Array) || (bytes !== undefined && body.byteLength !== bytes) ||
      await digest(body) !== sha256) throw new Error('Private cliff terrain asset differs from source');
  return body;
}

function packetReader(bytes) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  let offset = 0;
  return {
    number() {
      const value = view.getUint32(offset, true);
      offset += 4;
      return value;
    },
    slice(count) {
      if (!Number.isSafeInteger(count) || count < 0 || count > bytes.byteLength - offset) {
        throw new RangeError('Invalid cliff terrain packet bounds');
      }
      const from = offset;
      offset += count;
      return bytes.subarray(from, offset);
    },
    finish() {
      if (offset !== bytes.byteLength) throw new Error('Trailing cliff terrain packet words');
    },
  };
}

function readMaterial(bytes, detail, keys) {
  const input = packetReader(bytes);
  const n = () => input.number();
  if (n() !== 0x32334443 || n() !== 1 || n() !== detail || n() !== width || n() !== height || n() !== blockCount) {
    throw new Error('Invalid cliff material packet header');
  }
  const groupCount = n(), baseCount = n(), overlayCount = n(), indexCount = n();
  const blocks = Array.from({ length: blockCount }, () => [n(), n()]);
  const groups = Array.from({ length: groupCount }, () => ({ key: n(), baseIndex: n(), baseCount: n(), overlayIndex: n(), overlayCount: n() }));
  let groupOffset = 0, indexOffset = 0;
  for (const [first, count] of blocks) {
    if (first !== groupOffset || count < 1 || count > keys.size || first + count > groupCount) {
      throw new Error('Invalid cliff material block rows');
    }
    let last = -1;
    for (let i = first; i < first + count; ++i) {
      const row = groups[i];
      if (!keys.has(row.key) || row.key <= last || row.baseIndex !== indexOffset ||
          row.overlayIndex !== indexOffset + row.baseCount || row.baseCount % 3 || row.overlayCount % 3) {
        throw new Error('Invalid cliff material draw row');
      }
      last = row.key;
      indexOffset += row.baseCount + row.overlayCount;
    }
    groupOffset += count;
  }
  if (groupOffset !== groupCount || indexOffset !== indexCount) throw new Error('Incomplete cliff material draw rows');
  const base = input.slice(baseCount * 24), overlay = input.slice(overlayCount * 32), indices = input.slice(indexCount * 4);
  input.finish();
  return { blocks, groups, base, overlay, indices };
}

function readProjection(bytes) {
  const input = packetReader(bytes);
  const n = () => input.number();
  if (n() !== 0x32335043 || n() !== 1 || n() !== width || n() !== height || n() !== blockCount) {
    throw new Error('Invalid cliff projection packet header');
  }
  const vertexCount = n(), indexCount = n();
  const blocks = [];
  for (let block = 0, offset = 0; block < blockCount; ++block) {
    const first = n(), count = n();
    if (first !== offset || count < 1 || count % 3) throw new Error('Invalid cliff projection draw row');
    blocks.push({ first, count });
    offset += count;
    if (block === blockCount - 1 && offset !== indexCount) throw new Error('Incomplete cliff projection draw rows');
  }
  const vertices = input.slice(vertexCount * 32), indices = input.slice(indexCount * 4);
  input.finish();
  return { blocks, vertices, indices, indexCount };
}

const materialVertex = overlay => `#version 300 es
layout(location=0) in vec3 aNative;
layout(location=1) in vec4 aBGRA;
layout(location=2) in vec2 aUV;
${overlay ? 'layout(location=3) in vec2 aMaskUV;out vec2 vMaskUV;' : ''}
uniform mat4 uVP;
out vec2 vUV;
out vec3 vTint;
out vec2 vWorldXY;
out float vElevation;
invariant gl_Position;
void main(){
  vUV=aUV;
  vTint=aBGRA.bgr;
  vWorldXY=aNative.xy;
  vElevation=aNative.z;
  ${overlay ? 'vMaskUV=aMaskUV;' : ''}
  gl_Position=uVP*vec4(aNative.xzy,1.);
}`;

const materialFragment = overlay => `#version 300 es
precision highp float;
uniform sampler2D uColor;
${overlay ? 'uniform sampler2D uMask;in vec2 vMaskUV;' : ''}
uniform sampler2D uFog;
uniform float uTint;
uniform vec2 uFogSize;
uniform vec4 uClipPlane;
uniform bool uFogNow,uClip;
in vec2 vUV,vWorldXY;
in float vElevation;
in vec3 vTint;
out vec4 outColor;
void main(){
  if(uClip&&dot(vec4(vWorldXY.x,vElevation,vWorldXY.y,1.),uClipPlane)<0.)discard;
  vec4 source=texture(uColor,vUV);
  vec3 tint=mix(vec3(127./255.),vTint,uTint);
  vec3 material=clamp(2.*source.rgb*tint,0.,1.);
  float fog=uFogNow?texture(uFog,vWorldXY/uFogSize).r:1.;
  float reveal=smoothstep(.02,.45,fog)*mix(.5,1.,smoothstep(.55,.95,fog));
  outColor=vec4(material*reveal,${overlay ? 'texture(uMask,vMaskUV).r' : 'source.a'});
}`;

const lightVertex = `#version 300 es
layout(location=0) in vec3 aPosition;
layout(location=2) in vec2 aLightUV;
uniform mat4 uVP;
out vec2 vUV;
out vec3 vPosition;
invariant gl_Position;
void main(){vUV=aLightUV;vPosition=aPosition;gl_Position=uVP*vec4(aPosition,1.);}`;

const lightFragment = `#version 300 es
precision highp float;
uniform highp sampler2DArray uLightmap;
uniform float uLayer;
uniform vec3 uDiffuse,uAmbient;
uniform vec4 uClipPlane;
uniform bool uClip;
in vec2 vUV;
in vec3 vPosition;
out vec4 outColor;
void main(){
  if(uClip&&dot(vec4(vPosition,1.),uClipPlane)<0.)discard;
  float brightness=texture(uLightmap,vec3(vUV,uLayer)).r;
  vec3 first=clamp(2.*brightness*uDiffuse,0.,1.);
  outColor=vec4(clamp(first+uAmbient,0.,1.),1.);
}`;

function linkProgram(gl, vertex, fragment, names) {
  const program = gl.createProgram();
  try {
    for (const [type, source] of [[gl.VERTEX_SHADER, vertex], [gl.FRAGMENT_SHADER, fragment]]) {
      const shader = gl.createShader(type);
      try {
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
        gl.attachShader(program, shader);
      } finally {
        gl.deleteShader(shader);
      }
    }
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
    return { program, u: Object.fromEntries(names.map(name => [name, gl.getUniformLocation(program, 'u' + name)])) };
  } catch (error) {
    gl.deleteProgram(program);
    throw error;
  }
}

function withState(gl, units, run) {
  const get = name => gl.getParameter(gl[name]);
  const active = get('ACTIVE_TEXTURE');
  const textures = units.map(([unit, target]) => {
    gl.activeTexture(gl.TEXTURE0 + unit);
    return [unit, target, get(target === gl.TEXTURE_2D ? 'TEXTURE_BINDING_2D' : 'TEXTURE_BINDING_2D_ARRAY'), get('SAMPLER_BINDING')];
  });
  const state = {
    program: get('CURRENT_PROGRAM'), vao: get('VERTEX_ARRAY_BINDING'), depth: get('DEPTH_FUNC'),
    write: get('DEPTH_WRITEMASK'), cull: get('CULL_FACE_MODE'), winding: get('FRONT_FACE'),
    equation: [get('BLEND_EQUATION_RGB'), get('BLEND_EQUATION_ALPHA')],
    blend: ['BLEND_SRC_RGB', 'BLEND_DST_RGB', 'BLEND_SRC_ALPHA', 'BLEND_DST_ALPHA'].map(get),
    enabled: [gl.DEPTH_TEST, gl.CULL_FACE, gl.BLEND, gl.STENCIL_TEST, gl.POLYGON_OFFSET_FILL].map(cap => [cap, gl.isEnabled(cap)]),
  };
  try {
    for (const [unit] of units) gl.bindSampler(unit, null);
    return run();
  } finally {
    gl.useProgram(state.program);
    gl.bindVertexArray(state.vao);
    gl.depthFunc(state.depth);
    gl.depthMask(state.write);
    gl.cullFace(state.cull);
    gl.frontFace(state.winding);
    gl.blendEquationSeparate(...state.equation);
    gl.blendFuncSeparate(...state.blend);
    for (const [cap, on] of state.enabled) (on ? gl.enable : gl.disable).call(gl, cap);
    for (const [unit, target, texture, sampler] of textures) {
      gl.activeTexture(gl.TEXTURE0 + unit);
      gl.bindTexture(target, texture);
      gl.bindSampler(unit, sampler);
    }
    gl.activeTexture(active);
  }
}

function depthAndCull(gl, write, reverseCull, useStencil) {
  gl.enable(gl.DEPTH_TEST);
  gl.enable(gl.CULL_FACE);
  gl.disable(gl.POLYGON_OFFSET_FILL);
  if (!useStencil) gl.disable(gl.STENCIL_TEST);
  gl.depthFunc(gl.LEQUAL);
  gl.depthMask(write);
  gl.cullFace(gl.BACK);
  gl.frontFace(reverseCull ? gl.CW : gl.CCW);
}

function resolveSeason(value) {
  const yearLength = value?.yearLength ?? 1200;
  const elapsed = value?.elapsed ?? ((value?.month ?? 6) * yearLength / 12);
  const coordinate = f32(elapsed) / f32(yearLength) * 12;
  const bias = 103079215104;
  const current = Math.floor(coordinate + bias - bias);
  const month = value?.month ?? current;
  if (!Number.isFinite(yearLength) || !(elapsed >= 0 && elapsed < yearLength) || !Number.isInteger(month) || month < 0 || month >= 12 ||
      current < 0 || current >= 12) throw new RangeError('Invalid cliff material season');
  return { month, current, next: (current + 1) % 12, fraction: f32(f32(coordinate) - current) };
}

function tintAlpha(row, { current, next, fraction }) {
  return (Math.trunc((row.tint[current] * (1 - fraction) + row.tint[next] * fraction) * 255) & 255) / 255;
}

function activeBlocks(visible) {
  if (visible == null) return Array.from({ length: blockCount }, (_, i) => i);
  const rows = Array.from(visible);
  if (new Set(rows).size !== rows.length || rows.some(i => !Number.isInteger(i) || i < 0 || i >= blockCount)) {
    throw new RangeError('Invalid original cliff block selection');
  }
  return rows;
}

export async function loadAuthoredCliffTerrain(gl, geometry, { read = readLocal, detail = 2, season = null, highSampling = true } = {}) {
  if (!admitsAuthoredCliffTerrain(geometry) || !globalThis.crypto?.subtle) return null;
  const manifest = JSON.parse(new TextDecoder().decode(await checked(read, AUTHORED_CLIFF_TERRAIN)));
  if (manifest.format !== 'ee2-private-death-valley2-terrain-v1' || manifest.localOnly !== true ||
      manifest.sourceArchiveSha256 !== archive || manifest.cliffCount !== 453 ||
      manifest.dimensions.join(',') !== '100,300' || manifest.blocks.join(',') !== '4,12' ||
      await digest(geometry.elevations) !== manifest.elevationsSha256) throw new Error('Cliff terrain geometry differs from the archive');
  if (!Number.isInteger(detail) || manifest.details[detail]?.detail !== detail) throw new RangeError('Invalid cliff terrain detail');
  const selected = manifest.details[detail];
  const keys = new Set(selected.keys);
  const sources = new Map(manifest.materials.filter(row => keys.has(row.key)).map(row => [row.key, row]));
  if (sources.size !== keys.size) throw new Error('Incomplete original cliff texture definitions');
  const [materialBytes, projectionBytes, maskBytes] = await Promise.all([
    checked(read, selected), checked(read, manifest.geometry), checked(read, manifest.mask),
  ]);
  const material = readMaterial(materialBytes, detail, keys);
  const projection = readProjection(projectionBytes);
  const backing = geometry.backing ?? 512;
  if (!Number.isInteger(backing) || backing < height) throw new RangeError('Invalid cliff terrain fog backing');

  const resources = { programs: [], arrays: [], buffers: [], textures: [] };
  const colors = new Map();
  let disposed = false, revision = 0, phase;
  const make = (kind, create) => {
    const object = create();
    if (!object) throw new Error('Cannot allocate authored cliff renderer');
    resources[kind].push(object);
    return object;
  };
  function dispose() {
    if (disposed) return;
    disposed = true;
    for (const program of resources.programs) gl.deleteProgram(program);
    for (const array of resources.arrays) gl.deleteVertexArray(array);
    for (const buffer of resources.buffers) gl.deleteBuffer(buffer);
    for (const texture of resources.textures) gl.deleteTexture(texture);
  }
  function createBuffer(target, bytes, usage = gl.STATIC_DRAW) {
    const buffer = make('buffers', () => gl.createBuffer());
    gl.bindBuffer(target, buffer);
    gl.bufferData(target, bytes, usage);
    return buffer;
  }
  function vao(vertexBytes, element, stride, attributes) {
    const array = make('arrays', () => gl.createVertexArray());
    gl.bindVertexArray(array);
    const vertices = createBuffer(gl.ARRAY_BUFFER, vertexBytes);
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, element);
    for (const [location, size, kind, normalize, offset] of attributes) {
      gl.enableVertexAttribArray(location);
      gl.vertexAttribPointer(location, size, kind, normalize, stride, offset);
    }
    return { array, vertices };
  }
  function uploadMipChain(bytes, alpha) {
    const texture = make('textures', () => gl.createTexture());
    const target = gl.TEXTURE_2D, channels = alpha ? 1 : 4;
    const pixelState = [gl.UNPACK_ALIGNMENT, gl.UNPACK_ROW_LENGTH, gl.UNPACK_SKIP_PIXELS, gl.UNPACK_SKIP_ROWS];
    const previousPixels = pixelState.map(parameter => gl.getParameter(parameter));
    const previousBinding = gl.getParameter(gl.TEXTURE_BINDING_2D);
    try {
      gl.bindTexture(target, texture);
      pixelState.forEach((parameter, i) => gl.pixelStorei(parameter, i === 0 ? 1 : 0));
      gl.texStorage2D(target, 9, alpha ? gl.R8 : gl.RGBA8, 256, 256);
      let offset = 0;
      for (let level = 0, side = 256; level < 9; ++level, side >>= 1) {
        const end = offset + side * side * channels;
        gl.texSubImage2D(target, level, 0, 0, side, side, alpha ? gl.RED : gl.RGBA,
          gl.UNSIGNED_BYTE, bytes.subarray(offset, end));
        offset = end;
      }
      gl.texParameteri(target, gl.TEXTURE_MIN_FILTER, highSampling ? gl.LINEAR_MIPMAP_LINEAR : gl.LINEAR_MIPMAP_NEAREST);
      gl.texParameteri(target, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
      for (const axis of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T]) gl.texParameteri(target, axis, gl.REPEAT);
      const anisotropy = highSampling && gl.getExtension('EXT_texture_filter_anisotropic');
      if (anisotropy) gl.texParameterf(target, anisotropy.TEXTURE_MAX_ANISOTROPY_EXT,
        Math.min(4, gl.getParameter(anisotropy.MAX_TEXTURE_MAX_ANISOTROPY_EXT)));
    } finally {
      pixelState.forEach((parameter, i) => gl.pixelStorei(parameter, previousPixels[i]));
      gl.bindTexture(target, previousBinding);
    }
    return texture;
  }
  async function setSeason(next) {
    if (disposed) return false;
    const candidate = resolveSeason(next);
    const ticket = ++revision;
    const names = new Set(Array.from(sources.values(), row => row.months[candidate.month]));
    const pending = await Promise.all(Array.from(names, async name => {
      if (colors.has(name)) return null;
      return [name, await checked(read, manifest.textures[name])];
    }));
    if (disposed || ticket !== revision) return false;
    for (const item of pending) if (item && !colors.has(item[0])) colors.set(item[0], uploadMipChain(item[1], false));
    phase = candidate;
    return true;
  }

  try {
    const materialNames = ['VP', 'Color', 'Mask', 'Fog', 'FogSize', 'Tint', 'FogNow', 'Clip', 'ClipPlane'];
    const colorPrograms = [];
    for (const overlay of [false, true]) {
      const program = linkProgram(gl, materialVertex(overlay), materialFragment(overlay), materialNames);
      colorPrograms.push(program);
      resources.programs.push(program.program);
    }
    const light = linkProgram(gl, lightVertex, lightFragment, ['VP', 'Lightmap', 'Layer', 'Diffuse', 'Ambient', 'Clip', 'ClipPlane']);
    resources.programs.push(light.program);
    const previousArray = gl.getParameter(gl.VERTEX_ARRAY_BINDING);
    const previousBuffer = gl.getParameter(gl.ARRAY_BUFFER_BINDING);
    const scratch = make('arrays', () => gl.createVertexArray());
    let base, overlays, canonical, fogArray, fogBuffer;
    try {
      gl.bindVertexArray(scratch);
      const index = createBuffer(gl.ELEMENT_ARRAY_BUFFER, material.indices);
      const attributes = [[0, 3, gl.FLOAT, false, 0], [1, 4, gl.UNSIGNED_BYTE, true, 12], [2, 2, gl.FLOAT, false, 16]];
      base = vao(material.base, index, 24, attributes).array;
      overlays = vao(material.overlay, index, 32, [...attributes, [3, 2, gl.FLOAT, false, 24]]).array;
      gl.bindVertexArray(scratch);
      const projectionIndex = createBuffer(gl.ELEMENT_ARRAY_BUFFER, projection.indices);
      const mesh = vao(projection.vertices, projectionIndex, 32, [[0, 3, gl.FLOAT, false, 0], [2, 2, gl.FLOAT, false, 24]]);
      canonical = mesh.array;
      fogArray = make('arrays', () => gl.createVertexArray());
      gl.bindVertexArray(fogArray);
      gl.bindBuffer(gl.ARRAY_BUFFER, mesh.vertices);
      gl.enableVertexAttribArray(0);
      gl.vertexAttribPointer(0, 3, gl.FLOAT, false, 32, 0);
      fogBuffer = createBuffer(gl.ELEMENT_ARRAY_BUFFER, 0, gl.DYNAMIC_DRAW);
    } finally {
      gl.bindVertexArray(previousArray);
      gl.bindBuffer(gl.ARRAY_BUFFER, previousBuffer);
    }
    const mask = uploadMipChain(maskBytes, true);
    await setSeason(season);
    let lastFogSelection = '', fogCount = 0;
    const metrics = { baseDraws: 0, overlayDraws: 0, lightmapDraws: 0, fogDraws: 0, detail };
    return {
      geometryVao: canonical, indexCount: projection.indexCount, metrics, setSeason, dispose,
      draw(vp, fogTexture, { visibleBlocks, deferFog = false, reverseCull = false, clipPlane = null, fogSize = [backing,backing] } = {}) {
        if (disposed) return 0;
        const blocks = activeBlocks(visibleBlocks);
        metrics.baseDraws = metrics.overlayDraws = 0;
        return withState(gl, [[0, gl.TEXTURE_2D], [1, gl.TEXTURE_2D], [2, gl.TEXTURE_2D]], () => {
          depthAndCull(gl, true, reverseCull, false);
          if (clipPlane) gl.disable(gl.CULL_FACE);
          gl.enable(gl.BLEND);
          gl.blendEquation(gl.FUNC_ADD);
          gl.activeTexture(gl.TEXTURE0 + 1);
          gl.bindTexture(gl.TEXTURE_2D, mask);
          gl.activeTexture(gl.TEXTURE0 + 2);
          gl.bindTexture(gl.TEXTURE_2D, fogTexture ?? mask);
          for (const [index, layout] of ['base', 'overlay'].entries()) {
            const { program, u } = colorPrograms[index];
            gl.blendFunc(index ? gl.SRC_ALPHA : gl.ONE, index ? gl.ONE_MINUS_SRC_ALPHA : gl.ZERO);
            gl.useProgram(program);
            gl.uniformMatrix4fv(u.VP, false, vp);
            gl.uniform1i(u.Color, 0);
            gl.uniform1i(u.Mask, 1);
            gl.uniform1i(u.Fog, 2);
            gl.uniform1i(u.FogNow, Number(!deferFog && Boolean(fogTexture)));
            gl.uniform2fv(u.FogSize, fogSize);
            gl.uniform1i(u.Clip, Number(Boolean(clipPlane)));
            gl.uniform4fv(u.ClipPlane, clipPlane ?? [0,0,0,0]);
            gl.bindVertexArray(index ? overlays : base);
            gl.activeTexture(gl.TEXTURE0);
            for (const id of blocks) {
              const [first, count] = material.blocks[id];
              for (let i = first; i < first + count; ++i) {
                const row = material.groups[i], faces = row[layout + 'Count'];
                if (!faces) continue;
                const source = sources.get(row.key);
                gl.bindTexture(gl.TEXTURE_2D, colors.get(source.months[phase.month]));
                gl.uniform1f(u.Tint, tintAlpha(source, phase));
                gl.drawElements(gl.TRIANGLES, faces, gl.UNSIGNED_INT, row[layout + 'Index'] * 4);
                metrics[layout + 'Draws']++;
              }
            }
          }
          return metrics.baseDraws + metrics.overlayDraws;
        });
      },
      drawLightmap(vp, { texture, blocksX: suppliedBlocksX, diffuse, ambient, visibleBlocks, reverseCull = false, useStencil = false, clipPlane = null }) {
        if (disposed) return 0;
        if (!texture || suppliedBlocksX !== blocksX || diffuse?.length !== 3 || ambient?.length !== 3) {
          throw new RangeError('Invalid authored cliff lightmap');
        }
        const blocks = activeBlocks(visibleBlocks);
        metrics.lightmapDraws = 0;
        return withState(gl, [[0, gl.TEXTURE_2D_ARRAY]], () => {
          depthAndCull(gl, false, reverseCull, useStencil);
          if (clipPlane) gl.disable(gl.CULL_FACE);
          gl.enable(gl.BLEND);
          gl.blendEquation(gl.FUNC_ADD);
          gl.blendFuncSeparate(gl.DST_COLOR, gl.SRC_COLOR, gl.ZERO, gl.ONE);
          gl.useProgram(light.program);
          gl.uniformMatrix4fv(light.u.VP, false, vp);
          gl.uniform1i(light.u.Lightmap, 0);
          gl.uniform3fv(light.u.Diffuse, diffuse);
          gl.uniform3fv(light.u.Ambient, ambient);
          gl.uniform1i(light.u.Clip, Number(Boolean(clipPlane)));
          gl.uniform4fv(light.u.ClipPlane, clipPlane ?? [0,0,0,0]);
          gl.activeTexture(gl.TEXTURE0);
          gl.bindTexture(gl.TEXTURE_2D_ARRAY, texture);
          gl.bindVertexArray(canonical);
          for (const id of blocks) {
            const { first, count } = projection.blocks[id];
            gl.uniform1f(light.u.Layer, id);
            gl.drawElements(gl.TRIANGLES, count, gl.UNSIGNED_INT, first * 4);
            metrics.lightmapDraws++;
          }
          return metrics.lightmapDraws;
        });
      },
      drawLateFog(fog, vp, quality, { visibleBlocks, camera = 'main' } = {}) {
        if (disposed) return 0;
        let drawn;
        if (visibleBlocks == null) {
          drawn = fog.draw(camera, vp, canonical, quality, projection.indexCount);
        } else {
          const blocks = activeBlocks(visibleBlocks);
          if (!blocks.length) return 0;
          const selection = blocks.join(',');
          if (selection !== lastFogSelection) {
            fogCount = blocks.reduce((total, id) => total + projection.blocks[id].count, 0);
            const subset = new Uint8Array(fogCount * 4);
            for (let offset = 0, i = 0; i < blocks.length; ++i) {
              const { first, count } = projection.blocks[blocks[i]];
              const range = projection.indices.subarray(first * 4, (first + count) * 4);
              subset.set(range, offset);
              offset += range.length;
            }
            const previous = gl.getParameter(gl.VERTEX_ARRAY_BINDING);
            try {
              gl.bindVertexArray(fogArray);
              gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, fogBuffer);
              gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, subset, gl.DYNAMIC_DRAW);
            } finally {
              gl.bindVertexArray(previous);
            }
            lastFogSelection = selection;
          }
          drawn = fog.draw(camera, vp, fogArray, quality, fogCount);
        }
        if (drawn) metrics.fogDraws++;
        return drawn;
      },
    };
  } catch (error) {
    dispose();
    throw error;
  }
}
