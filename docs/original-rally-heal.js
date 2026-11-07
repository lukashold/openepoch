// Original play-once Rally mesh. Events and the actual parent scale come from the current match.
import {saveSceneGLState, restoreSceneGLState} from './original-reflection.js';
import {samplePriestKeyGroup} from './original-priest-fx.js';
import {DEFAULT_MODEL_LIGHTING, modelMaterial, alphaState, depthState} from './original-material.js';

export const RALLY_HEAL_DURATION = 1;
const folder = 'local-original/rally-heal/';
const vertex = `#version 300 es
layout(location=0) in vec3 aPosition;
layout(location=1) in vec3 aNormal;
layout(location=2) in vec2 aUV;
uniform mat4 uVP;
uniform vec3 uEye,uAmbient,uSun,uSpecularLight,uDirection;
uniform vec3 uMaterialAmbient,uMaterialDiffuse,uMaterialEmission,uMaterialSpecular;
uniform float uGlossiness;
out vec2 vUV;
out vec3 vDiffuse,vSpecular;
void main(){
 vec3 n=normalize(aNormal), l=normalize(uDirection);
 float facing=max(dot(n,l),0.);
 vDiffuse=clamp(uMaterialEmission+uMaterialAmbient*uAmbient+uMaterialDiffuse*uSun*facing,0.,1.);
 float shine=facing>0.?pow(max(dot(n,normalize(l+normalize(uEye-aPosition))),0.),uGlossiness):0.;
 vSpecular=uMaterialSpecular*uSpecularLight*shine;
 vUV=aUV;
 gl_Position=uVP*vec4(aPosition,1.);
}`;
const fragment = `#version 300 es
precision highp float;
uniform sampler2D uBase;
uniform float uAlpha;
in vec2 vUV;
in vec3 vDiffuse,vSpecular;
out vec4 frag;
void main(){
 vec4 texel=texture(uBase,vUV);
 frag=vec4(clamp(texel.rgb*vDiffuse+vSpecular,0.,1.),texel.a*uAlpha);
}`;

const finite = (value, size) => value?.length === size && Array.from(value).every(Number.isFinite);
const mul = (vector, factor) => vector.map(value => value * factor);
const add = (a, b) => a.map((value, index) => value + b[index]);
const rotate = (vector, matrix) => [0, 3, 6].map(row =>
  vector[0] * matrix[row] + vector[1] * matrix[row + 1] + vector[2] * matrix[row + 2]);
const spin = ([x, y, z], sin, cos) => [x * cos - y * sin, x * sin + y * cos, z];
const world = ([x, y, z]) => [x, z, -y];

export function projectRallyHeal(mesh, event) {
  if (event?.kind !== 'healed' || event.visible !== true || !finite(event.position, 3)
      || !Number.isFinite(event.yaw) || !Number.isFinite(event.scale) || !(event.scale > 0)
      || !(event.elapsed >= 0 && event.elapsed < RALLY_HEAL_DURATION)) return null;
  const angle = samplePriestKeyGroup(mesh.rotationZ, event.elapsed)[0];
  const [sin, cos] = [Math.sin(angle), Math.cos(angle)];
  const [actorSin, actorCos] = [Math.sin(-event.yaw), Math.cos(-event.yaw)];
  const actor = point => world(spin(point, actorSin, actorCos));
  const root = point => add(mesh.root.translation, mul(rotate(point, mesh.root.rotation), mesh.root.scale));
  const child = point => add(mesh.parent.translation, mul(spin(point, sin, cos), mesh.parent.scale));
  return {
    positions: mesh.positions.map(point => add(event.position,
      mul(actor(root(child(add(mesh.shape.translation, mul(rotate(point, mesh.shape.rotation), mesh.shape.scale))))), event.scale))),
    normals: mesh.normals.map(normal => actor(rotate(spin(rotate(normal, mesh.shape.rotation), sin, cos), mesh.root.rotation))),
    angle,
  };
}

export function createRallyHealGate() {
  let known = new Map(), attached = new Map();
  return {
    advance(events, previousViews, now) {
      const next = new Map(), accepted = [];
      const beforeRetirement = new Map(attached);
      for (const event of events) {
        if (event?.kind !== 'healed' || !Number.isFinite(event.started) || !Number.isInteger(event.handle)) continue;
        const key = `${event.handle}:${event.started}`;
        if (next.has(key)) continue;
        const admitted = known.get(key) ?? (!beforeRetirement.has(event.handle)
          && previousViews.some(snapshot => snapshot.forHandle(event.handle)));
        next.set(key, admitted);
        if (admitted) {
          beforeRetirement.set(event.handle, key);
          if (now - event.started < RALLY_HEAL_DURATION) accepted.push({...event, key});
        }
      }
      known = next;
      attached = new Map(accepted.map(event => [event.handle, event.key]));
      return accepted;
    },
    reset() { known.clear(); attached.clear(); },
  };
}

function shaderProgram(gl) {
  const program = gl.createProgram();
  try {
    for (const [type, text] of [[gl.VERTEX_SHADER, vertex], [gl.FRAGMENT_SHADER, fragment]]) {
      const shader = gl.createShader(type);
      try {
        gl.shaderSource(shader, text); gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw Error(gl.getShaderInfoLog(shader));
        gl.attachShader(program, shader);
      } finally { gl.deleteShader(shader); }
    }
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw Error(gl.getProgramInfoLog(program));
    return program;
  } catch (error) { gl.deleteProgram(program); throw error; }
}

export async function loadOriginalRallyHeal(gl, {enabled = false, base = new URL('./', import.meta.url),
    fetchFile = globalThis.fetch, decode = globalThis.createImageBitmap} = {}) {
  if (!enabled) return null;
  let image;
  const release = [];
  try {
    const response = await fetchFile(new URL(folder + 'manifest.json', base));
    if (!response.ok) return null;
    const manifest = await response.json();
    const {mesh, texture} = manifest;
    if (manifest.format !== 'ee2-native-rally-heal-v1' || manifest.duration !== RALLY_HEAL_DURATION
        || texture.url !== folder + 'fx_healing.png' || mesh.positions.length !== 48 || mesh.normals.length !== 48
        || mesh.uvs.length !== 48 || mesh.indices.length !== 72
        || mesh.indices.some(index => !Number.isInteger(index) || index < 0 || index >= 48)) return null;
    const nativeProperties = {NiVertexColorProperty: mesh.color, NiSpecularProperty: mesh.specular,
      NiAlphaProperty: mesh.alpha, NiZBufferProperty: mesh.depth};
    const rgb = record => ['r', 'g', 'b'].map(key => record[key]);
    const material = modelMaterial({nativeProperties, material: {
      ambient: rgb(mesh.material['Ambient Color']), diffuse: rgb(mesh.material['Diffuse Color']),
      emissive: rgb(mesh.material['Emissive Color']), specular: rgb(mesh.material['Specular Color']),
      glossiness: mesh.material.Glossiness, alpha: mesh.material.Alpha,
    }});
    const alpha = alphaState({nativeProperties}), depth = depthState({nativeProperties});
    if (alpha.flags !== 237 || alpha.test || !depth.test || !depth.write || depth.function !== 3
        || !material.specularEnabled || material.vertexMode !== 0 || material.lightingMode !== 1
        || mesh.baseTexture['Clamp Mode'] !== 3 || mesh.baseTexture['Filter Mode'] !== 2) return null;
    const file = await fetchFile(new URL(texture.url, base));
    if (!file.ok) return null;
    image = await decode(await file.blob(), {premultiplyAlpha: 'none', colorSpaceConversion: 'none'});
    if (image.width !== texture.size[0] || image.height !== texture.size[1]) return null;

    const state = saveSceneGLState(gl), flip = gl.getParameter(gl.UNPACK_FLIP_Y_WEBGL);
    try {
      const program = shaderProgram(gl);
      release.push(() => gl.deleteProgram(program));
      const uniforms = Object.fromEntries(['uVP', 'uEye', 'uAmbient', 'uSun', 'uSpecularLight', 'uDirection',
        'uMaterialAmbient', 'uMaterialDiffuse', 'uMaterialEmission', 'uMaterialSpecular', 'uGlossiness', 'uBase', 'uAlpha']
        .map(name => [name, gl.getUniformLocation(program, name)]));
      const tex = gl.createTexture(), vao = gl.createVertexArray(), points = gl.createBuffer(), indices = gl.createBuffer();
      release.push(() => { gl.deleteTexture(tex); gl.deleteVertexArray(vao); gl.deleteBuffer(points); gl.deleteBuffer(indices); });
      gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tex);
      gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
      for (const flag of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, flag, gl.LINEAR);
      for (const flag of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T]) gl.texParameteri(gl.TEXTURE_2D, flag, gl.REPEAT);
      gl.bindVertexArray(vao); gl.bindBuffer(gl.ARRAY_BUFFER, points);
      for (const [slot, length, offset] of [[0, 3, 0], [1, 3, 12], [2, 2, 24]]) {
        gl.enableVertexAttribArray(slot); gl.vertexAttribPointer(slot, length, gl.FLOAT, false, 32, offset);
      }
      gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER, indices);
      gl.bufferData(gl.ELEMENT_ARRAY_BUFFER, new Uint16Array(mesh.indices), gl.STATIC_DRAW);
      const buffer = new Float32Array(48 * 8);
      mesh.uvs.forEach((uv, index) => buffer.set(uv, index * 8 + 6));
      let disposed = false;
      return {
        draw({vp, eye, lighting = DEFAULT_MODEL_LIGHTING, effects = []}) {
          if (disposed || !effects.length || !finite(vp, 16) || !finite(eye, 3)) return 0;
          const previous = saveSceneGLState(gl);
          try {
            gl.useProgram(program); gl.uniformMatrix4fv(uniforms.uVP, false, vp);
            for (const [name, value] of Object.entries({uEye: eye, uAmbient: lighting.ambient,
              uSun: lighting.sun, uSpecularLight: lighting.specular ?? lighting.sun, uDirection: lighting.direction,
              uMaterialAmbient: material.ambient, uMaterialDiffuse: material.diffuse,
              uMaterialEmission: material.emissive, uMaterialSpecular: material.specular})) gl.uniform3fv(uniforms[name], value);
            gl.uniform1f(uniforms.uGlossiness, material.glossiness); gl.uniform1f(uniforms.uAlpha, material.alpha);
            gl.uniform1i(uniforms.uBase, 0); gl.activeTexture(gl.TEXTURE0); gl.bindTexture(gl.TEXTURE_2D, tex);
            gl.enable(gl.DEPTH_TEST); gl.depthFunc(gl.LEQUAL); gl.depthMask(true);
            gl.enable(gl.CULL_FACE); gl.frontFace(gl.CCW); gl.cullFace(gl.BACK);
            gl.enable(gl.BLEND); gl.blendEquation(gl.FUNC_ADD); gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);
            gl.bindVertexArray(vao); gl.bindBuffer(gl.ARRAY_BUFFER, points);
            let count = 0;
            for (const event of effects) {
              const pose = projectRallyHeal(mesh, event);
              if (!pose) continue;
              pose.positions.forEach((position, index) => {
                buffer.set(position, index * 8); buffer.set(pose.normals[index], index * 8 + 3);
              });
              gl.bufferData(gl.ARRAY_BUFFER, buffer, gl.STREAM_DRAW);
              gl.drawElements(gl.TRIANGLES, 72, gl.UNSIGNED_SHORT, 0);
              count++;
            }
            return count;
          } finally { restoreSceneGLState(gl, previous); }
        },
        dispose() { if (!disposed) { disposed = true; release.reverse().forEach(remove => remove()); } },
      };
    } finally { gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, flip); restoreSceneGLState(gl, state); }
  } catch { release.reverse().forEach(remove => remove()); return null; }
  finally { image?.close(); }
}
