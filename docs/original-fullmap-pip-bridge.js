import {createOriginalPipCamera, originalPipContains, ORIGINAL_PIP_CAMERA_PROFILE} from './original-pip-camera.js';
import {createOriginalPipRenderer} from './original-pip-renderer.js';
import {updateOriginalCamera} from './original-camera.js';

const VS = `#version 300 es
out vec2 uv;
void main(){
 uv=vec2(float(gl_VertexID&1),float(gl_VertexID>>1));
 gl_Position=vec4(uv*2.-1.,0.,1.);
}`;
const FS = `#version 300 es
precision highp float;
in vec2 uv;
uniform sampler2D uPreview;
out vec4 color;
void main(){color=texture(uPreview,uv);}`;

function createPresenter(gl) {
  const program = gl.createProgram();
  for (const [type, source] of [[gl.VERTEX_SHADER, VS], [gl.FRAGMENT_SHADER, FS]]) {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader));
    gl.attachShader(program, shader);
    gl.deleteShader(shader);
  }
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(program));
  const sampler = gl.getUniformLocation(program, 'uPreview'), vao = gl.createVertexArray();
  return (texture, rect) => {
    const viewport = gl.getParameter(gl.VIEWPORT), active = gl.getParameter(gl.ACTIVE_TEXTURE);
    const color = gl.getParameter(gl.COLOR_WRITEMASK), programBefore = gl.getParameter(gl.CURRENT_PROGRAM);
    const vaoBefore = gl.getParameter(gl.VERTEX_ARRAY_BINDING);
    const flags = [gl.BLEND, gl.DEPTH_TEST, gl.STENCIL_TEST, gl.SCISSOR_TEST, gl.CULL_FACE].map(cap => [cap, gl.isEnabled(cap)]);
    gl.activeTexture(gl.TEXTURE0);
    const textureBefore = gl.getParameter(gl.TEXTURE_BINDING_2D);
    try {
      for (const [cap] of flags) gl.disable(cap);
      gl.colorMask(true, true, true, true);
      gl.viewport(rect.x, rect.y, rect.width, rect.height);
      gl.useProgram(program);
      gl.bindVertexArray(vao);
      gl.bindTexture(gl.TEXTURE_2D, texture);
      gl.uniform1i(sampler, 0);
      gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    } finally {
      gl.bindTexture(gl.TEXTURE_2D, textureBefore);
      gl.activeTexture(active);
      gl.useProgram(programBefore);
      gl.bindVertexArray(vaoBefore);
      gl.viewport(...viewport);
      gl.colorMask(...color);
      for (const [cap, enabled] of flags) (enabled ? gl.enable : gl.disable).call(gl, cap);
    }
  };
}

export function fullmapPipDeviceRect(viewport, canvasRect, width, height) {
  if (!viewport || ![viewport.x, viewport.y, viewport.width, viewport.height, canvasRect.x, canvasRect.y,
    canvasRect.width, canvasRect.height, width, height].every(Number.isFinite) ||
    Math.min(viewport.width, viewport.height, canvasRect.width, canvasRect.height, width, height) <= 0) return null;
  const sx = width / canvasRect.width, sy = height / canvasRect.height;
  const left = Math.round((viewport.x - canvasRect.x) * sx);
  const right = Math.round((viewport.x + viewport.width - canvasRect.x) * sx);
  const top = Math.round((viewport.y - canvasRect.y) * sy);
  const bottom = Math.round((viewport.y + viewport.height - canvasRect.y) * sy);
  if (left < 0 || top < 0 || right > width || bottom > height || left >= right || top >= bottom) return null;
  return {x: left, y: height - bottom, width: right - left, height: bottom - top};
}

// The map controller owns pointer coordinates, pane UI and its DOM aperture.
// The game owns the main view. This bridge owns the secondary presentation only.
export function createFullmapPipBridge({gl, canvas, available, mainPose, setMainFocus, resolveUnit, heightAt, onOpen, onClose}) {
  let camera = null, opened = null, held = null, wheelUnits = 0, renderer = null, present = null;
  const clearInput = () => { held = null; wheelUnits = 0; };
  const close = () => {
    if (!opened) return false;
    opened = null;
    clearInput();
    onClose?.();
    return true;
  };
  const point = value => Array.isArray(value) && value.length === 2 && value.every(Number.isFinite);
  const viewport=()=>opened&&fullmapPipDeviceRect(opened.getViewport(),canvas.getBoundingClientRect(),canvas.width,canvas.height);
  function advanceFrame(delta) {
    if (!opened || !available()) return null;
    const rect = viewport();
    if (!rect) return null;
    let pose = camera.update(delta, resolveUnit);
    if (held) { camera.focusMap(...held); pose = camera.pose(); }
    if (wheelUnits) { pose = camera.wheel(wheelUnits); wheelUnits = 0; }
    return {rect,pose};
  }
  return Object.freeze({
    available,
    isOpen: () => opened !== null,
    get pane() { return opened?.pane ?? null; },
    open({pane, getViewport = () => null, getPreviewCanvas = () => null, initialPip} = {}) {
      if (!available()) return false;
      if (typeof pane !== 'string' || !pane || typeof getViewport !== 'function' || typeof getPreviewCanvas !== 'function') {
        throw new TypeError('Invalid full-map PIP opening');
      }
      if (!camera) camera = createOriginalPipCamera(initialPip ?? {...mainPose(), nativeFraction: ORIGINAL_PIP_CAMERA_PROFILE.defaultFraction});
      const first = !opened;
      opened = {pane, getViewport, getPreviewCanvas};
      if (first) onOpen?.();
      return true;
    },
    focusWorld(value) {
      if (!opened || !point(value)) return false;
      camera.focusMap(...value);
      return true;
    },
    preview(value) {
      if (!opened || (value !== null && !point(value))) return false;
      held = value === null ? null : [...value];
      return true;
    },
    accept(value) {
      if (!opened || !point(value)) return false;
      setMainFocus(...value);
      return close();
    },
    close,
    reset() {
      close();
      camera = null;
      if (renderer) { renderer.dispose(); renderer = null; }
    },
    drag(dx, dy) {
      if (!opened?.getViewport()) return false;
      camera.drag(dx, dy);
      return true;
    },
    wheel(clientX, clientY, nativeUnits) {
      const viewport = opened?.getViewport();
      if (!viewport || !originalPipContains(viewport, clientX, clientY)) return false;
      if (!Number.isInteger(nativeUnits)) throw new RangeError('Expected native PIP wheel units');
      wheelUnits += nativeUnits;
      return true;
    },
    bookmark(slot, action, {time, unitHandle} = {}) {
      if (!opened?.getViewport()) return null;
      if (action === 'clear') return camera.clearBookmark(slot);
      if (action === 'select') return camera.selectBookmark(slot);
      const unit = Number.isInteger(unitHandle) && resolveUnit(unitHandle) ? {handle: unitHandle} : null;
      const options = {main: mainPose(), fullmap: true, unit};
      if (action === 'set') return camera.setBookmark(slot, options);
      if (action !== 'select-or-set') throw new RangeError('Unknown PIP bookmark action');
      const result = camera.selectOrSet(slot, time, options);
      if (result?.action === 'main-focus') setMainFocus(result.x, result.y);
      return result;
    },
    cycle() { return opened?.getViewport() ? camera.toggleCycle() : false; },
    snapshot: () => camera?.snapshot() ?? null,
    admissionProjection() {
      if (!opened || !available()) return null;
      const rect=viewport();
      if (!rect) return null;
      const pose=camera.pose();
      if (held) [pose.x,pose.y]=held;
      return updateOriginalCamera(pose,{},rect.width,rect.height,heightAt,ORIGINAL_PIP_CAMERA_PROFILE).vp;
    },
    prepare(delta) {
      const frame = advanceFrame(delta);
      if (!frame) return null;
      const {pose,rect} = frame;
      const view = updateOriginalCamera({...pose},{},rect.width,rect.height,heightAt,ORIGINAL_PIP_CAMERA_PROFILE);
      return {...frame,vp:view.vp};
    },
    draw(delta, drawWorld, prepared) {
      const frame = prepared === undefined ? advanceFrame(delta) : prepared;
      if (!frame || !opened || !available()) return null;
      const {rect,pose} = frame;
      renderer ??= createOriginalPipRenderer(gl);
      present ??= createPresenter(gl);
      const result = renderer.render({camera: pose, width: rect.width, height: rect.height, heightAt,
        drawWorld: view => drawWorld(view, pose)});
      present(result.texture, rect);
      const preview = opened.getPreviewCanvas();
      if (preview) {
        if (preview.width !== rect.width) preview.width = rect.width;
        if (preview.height !== rect.height) preview.height = rect.height;
        // A native dialog is in the top layer. Capture its canvas synchronously
        // while the main WebGL drawing buffer still contains the preview.
        const context = preview.getContext('2d', {alpha: false});
        context.drawImage(canvas, rect.x, canvas.height - rect.y - rect.height,
          rect.width, rect.height, 0, 0, rect.width, rect.height);
      }
      return {rect, view: result.view};
    },
  });
}
