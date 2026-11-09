export const TERRITORY_BORDER_GEOMETRY_STRIDE = 9;
const atlasLayers = 30;

const vertexSource = `#version 300 es
layout(location=0) in vec3 aPosition;
layout(location=1) in vec2 aUV;
layout(location=2) in float aLayer;
layout(location=3) in vec3 aTint;
uniform mat4 uVP;
out vec3 vWorld;
out vec2 vUV;
flat out float vLayer;
flat out vec3 vTint;
void main() {
  vWorld = aPosition;
  vUV = aUV;
  vLayer = aLayer;
  vTint = aTint;
  gl_Position = uVP * vec4(aPosition, 1.0);
}`;

const fragmentSource = `#version 300 es
precision highp float;
uniform highp sampler2DArray uTerritoryBorderMips;
uniform sampler2D uFog;
uniform bool uClipEnabled,uDeferFog,uDeferAllFog;
uniform float uDeferFogAbove;
uniform vec4 uClipPlane;
in vec3 vWorld;
in vec2 vUV;
flat in float vLayer;
flat in vec3 vTint;
out vec4 outColor;
void main() {
  float alpha = texture(uTerritoryBorderMips, vec3(vUV, vLayer)).r;
  float fog = uDeferFog && (uDeferAllFog || vWorld.y >= uDeferFogAbove) ? 1.0 :
    texture(uFog, clamp(vWorld.xz / 256.0, 0.0, 1.0)).r;
  if (fog < 0.015 || (uClipEnabled && dot(vec4(vWorld, 1.0), uClipPlane) < 0.0)) discard;
  float reveal = smoothstep(0.02, 0.45, fog) * mix(0.5, 1.0, smoothstep(0.55, 0.95, fog));
  outColor = vec4(vTint * (alpha * reveal), alpha);
}`;

function createProgram(gl) {
  const program = gl.createProgram();
  if (!program) throw Error('Cannot allocate territory border program');
  try {
    for (const [type, source] of [[gl.VERTEX_SHADER, vertexSource], [gl.FRAGMENT_SHADER, fragmentSource]]) {
      const shader = gl.createShader(type);
      if (!shader) throw Error('Cannot allocate territory border shader');
      try {
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw Error(gl.getShaderInfoLog(shader));
        gl.attachShader(program, shader);
      } finally {
        gl.deleteShader(shader);
      }
    }
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw Error(gl.getProgramInfoLog(program));
    return program;
  } catch (error) {
    gl.deleteProgram(program);
    throw error;
  }
}

function validatePacket(vertices, indices) {
  const stride = TERRITORY_BORDER_GEOMETRY_STRIDE;
  if (!(vertices instanceof Float32Array) || vertices.length % stride ||
      !(indices instanceof Uint32Array) || indices.length % 3) throw Error('Invalid territory border packet');
  for (let i = 0; i < vertices.length; i += stride) {
    for (let j = 0; j < stride; ++j) {
      if (!Number.isFinite(vertices[i + j])) throw Error('Nonfinite territory border attribute');
    }
    const layer = vertices[i + 5];
    if (!Number.isInteger(layer) || layer < 0 || layer >= atlasLayers) throw Error('Invalid territory border layer');
    for (let j = 6; j < 9; ++j) {
      if (vertices[i + j] < 0 || vertices[i + j] > 1) throw Error('Expected normalized territory border tint');
    }
  }
  for (const index of indices) {
    if (index >= vertices.length / stride) throw Error('Territory border index exceeds vertex packet');
  }
}

function sampler(gl, minFilter) {
  const value = gl.createSampler();
  if (!value) throw Error('Cannot allocate territory border sampler');
  gl.samplerParameteri(value, gl.TEXTURE_MIN_FILTER, minFilter);
  gl.samplerParameteri(value, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.samplerParameteri(value, gl.TEXTURE_WRAP_S, gl.MIRRORED_REPEAT);
  gl.samplerParameteri(value, gl.TEXTURE_WRAP_T, gl.MIRRORED_REPEAT);
  return value;
}

/** Draw one ordered stock geometry packet over existing terrain depth and color. */
export function createTerritoryBorderGeometryRenderer(gl, atlasTexture) {
  if (!gl.isTexture(atlasTexture)) throw Error('Expected an uploaded territory border atlas');
  const program = createProgram(gl);
  const vao = gl.createVertexArray();
  const verticesBuffer = gl.createBuffer();
  const indicesBuffer = gl.createBuffer();
  const samplers = [];
  let indexCount = 0;
  let disposed = false;

  function dispose() {
    if (disposed) return;
    disposed = true;
    gl.deleteVertexArray(vao);
    gl.deleteBuffer(verticesBuffer);
    gl.deleteBuffer(indicesBuffer);
    gl.deleteProgram(program);
    for (const value of samplers) gl.deleteSampler(value);
  }

  try {
    if (!vao || !verticesBuffer || !indicesBuffer) throw Error('Cannot allocate territory border geometry');
    samplers.push(sampler(gl, gl.LINEAR_MIPMAP_NEAREST));
    samplers.push(sampler(gl, gl.LINEAR_MIPMAP_LINEAR));
    const previousVao = gl.getParameter(gl.VERTEX_ARRAY_BINDING);
    const previousArray = gl.getParameter(gl.ARRAY_BUFFER_BINDING);
    try {
      gl.bindVertexArray(vao);
      gl.bindBuffer(gl.ARRAY_BUFFER, verticesBuffer);
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, indicesBuffer);
      const bytes = Float32Array.BYTES_PER_ELEMENT;
      const stride = TERRITORY_BORDER_GEOMETRY_STRIDE * bytes;
      for (const [location, size, offset] of [[0, 3, 0], [1, 2, 3], [2, 1, 5], [3, 3, 6]]) {
        gl.enableVertexAttribArray(location);
        gl.vertexAttribPointer(location, size, gl.FLOAT, false, stride, offset * bytes);
      }
    } finally {
      gl.bindVertexArray(previousVao);
      gl.bindBuffer(gl.ARRAY_BUFFER, previousArray);
    }
  } catch (error) {
    dispose();
    throw error;
  }

  const uniforms = Object.fromEntries(['uVP', 'uTerritoryBorderMips', 'uFog', 'uClipEnabled', 'uClipPlane', 'uDeferFog', 'uDeferFogAbove', 'uDeferAllFog']
    .map(name => [name, gl.getUniformLocation(program, name)]));

  function upload({ vertices, indices }) {
    if (disposed) throw Error('Territory border renderer is disposed');
    validatePacket(vertices, indices);
    // COPY_WRITE_BUFFER changes neither the caller's VAO nor its array-buffer binding.
    const previous = gl.getParameter(gl.COPY_WRITE_BUFFER_BINDING);
    try {
      gl.bindBuffer(gl.COPY_WRITE_BUFFER, verticesBuffer);
      gl.bufferData(gl.COPY_WRITE_BUFFER, vertices, gl.DYNAMIC_DRAW);
      gl.bindBuffer(gl.COPY_WRITE_BUFFER, indicesBuffer);
      gl.bufferData(gl.COPY_WRITE_BUFFER, indices, gl.DYNAMIC_DRAW);
      indexCount = indices.length;
      return indexCount;
    } finally {
      gl.bindBuffer(gl.COPY_WRITE_BUFFER, previous);
    }
  }

  function draw({ vp, fogTexture, highQuality = false, clipPlane = null, deferFogAbove = null, deferAllFog = false }) {
    if (disposed) throw Error('Territory border renderer is disposed');
    if (indexCount === 0) return 0;
    if (vp?.length !== 16 || !gl.isTexture(fogTexture) || (clipPlane && clipPlane.length !== 4))
      throw Error('Invalid territory border scene');

    const savedProgram = gl.getParameter(gl.CURRENT_PROGRAM);
    const savedVao = gl.getParameter(gl.VERTEX_ARRAY_BINDING);
    const savedTextureUnit = gl.getParameter(gl.ACTIVE_TEXTURE);
    const savedDepthFunc = gl.getParameter(gl.DEPTH_FUNC);
    const savedDepthMask = gl.getParameter(gl.DEPTH_WRITEMASK);
    const savedBlendEquation = [gl.BLEND_EQUATION_RGB, gl.BLEND_EQUATION_ALPHA].map(param => gl.getParameter(param));
    const savedBlendFunc = [gl.BLEND_SRC_RGB, gl.BLEND_DST_RGB, gl.BLEND_SRC_ALPHA, gl.BLEND_DST_ALPHA]
      .map(param => gl.getParameter(param));
    const caps = [gl.BLEND, gl.DEPTH_TEST, gl.CULL_FACE, gl.POLYGON_OFFSET_FILL]
      .map(cap => [cap, gl.isEnabled(cap)]);
    const textures = [];
    try {
      for (const [unit, target, binding, texture, sample] of [
        [0, gl.TEXTURE_2D_ARRAY, gl.TEXTURE_BINDING_2D_ARRAY, atlasTexture, samplers[Number(!!highQuality)]],
        [1, gl.TEXTURE_2D, gl.TEXTURE_BINDING_2D, fogTexture, null],
      ]) {
        gl.activeTexture(gl.TEXTURE0 + unit);
        textures.push([unit, target, gl.getParameter(binding), gl.getParameter(gl.SAMPLER_BINDING)]);
        gl.bindTexture(target, texture);
        gl.bindSampler(unit, sample);
      }
      gl.enable(gl.BLEND);
      gl.enable(gl.DEPTH_TEST);
      gl.disable(gl.CULL_FACE);
      gl.disable(gl.POLYGON_OFFSET_FILL);
      gl.depthFunc(gl.LEQUAL);
      gl.depthMask(false);
      gl.blendEquation(gl.FUNC_ADD);
      gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);

      gl.useProgram(program);
      gl.bindVertexArray(vao);
      gl.uniformMatrix4fv(uniforms.uVP, false, vp);
      gl.uniform1i(uniforms.uTerritoryBorderMips, 0);
      gl.uniform1i(uniforms.uFog, 1);
      gl.uniform1i(uniforms.uDeferFog, Number(deferAllFog || deferFogAbove !== null));
      gl.uniform1i(uniforms.uDeferAllFog, Number(deferAllFog));
      gl.uniform1f(uniforms.uDeferFogAbove, deferFogAbove ?? 0);
      gl.uniform1i(uniforms.uClipEnabled, Number(!!clipPlane));
      if (clipPlane) gl.uniform4fv(uniforms.uClipPlane, clipPlane);
      gl.drawElements(gl.TRIANGLES, indexCount, gl.UNSIGNED_INT, 0);
      return indexCount;
    } finally {
      for (const [unit, target, texture, sample] of textures) {
        gl.activeTexture(gl.TEXTURE0 + unit);
        gl.bindTexture(target, texture);
        gl.bindSampler(unit, sample);
      }
      gl.activeTexture(savedTextureUnit);
      gl.bindVertexArray(savedVao);
      gl.useProgram(savedProgram);
      gl.depthFunc(savedDepthFunc);
      gl.depthMask(savedDepthMask);
      gl.blendEquationSeparate(...savedBlendEquation);
      gl.blendFuncSeparate(...savedBlendFunc);
      for (const [cap, enabled] of caps) {
        if (enabled) gl.enable(cap);
        else gl.disable(cap);
      }
    }
  }

  return { upload, draw, dispose };
}
