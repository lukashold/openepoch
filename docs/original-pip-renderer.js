import {updateOriginalCamera} from './original-camera.js';
import {saveSceneGLState, restoreSceneGLState} from './original-reflection.js';
import {ORIGINAL_PIP_CAMERA_PROFILE} from './original-pip-camera.js';

// The native full-map viewport binds the independent PIP camera (0075B630,
// 007AE700, 0051EE90). The WebGL target is this port's presentation adapter.
export function createOriginalPipRenderer(gl) {
  let target = null, rendering = false, disposed = false;
  const retired = [];
  const destroy = row => {
    gl.deleteFramebuffer(row.framebuffer);
    gl.deleteRenderbuffer(row.depth);
    gl.deleteTexture(row.texture);
  };
  function allocate(width, height) {
    if (target?.width === width && target.height === height) return;
    const next = {width, height, texture: gl.createTexture(), framebuffer: gl.createFramebuffer(), depth: gl.createRenderbuffer()};
    try {
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, next.texture);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, width, height, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
      for (const key of [gl.TEXTURE_MIN_FILTER, gl.TEXTURE_MAG_FILTER]) gl.texParameteri(gl.TEXTURE_2D, key, gl.LINEAR);
      for (const key of [gl.TEXTURE_WRAP_S, gl.TEXTURE_WRAP_T]) gl.texParameteri(gl.TEXTURE_2D, key, gl.CLAMP_TO_EDGE);
      gl.bindFramebuffer(gl.FRAMEBUFFER, next.framebuffer);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, next.texture, 0);
      gl.bindRenderbuffer(gl.RENDERBUFFER, next.depth);
      gl.renderbufferStorage(gl.RENDERBUFFER, gl.DEPTH24_STENCIL8, width, height);
      gl.framebufferRenderbuffer(gl.FRAMEBUFFER, gl.DEPTH_STENCIL_ATTACHMENT, gl.RENDERBUFFER, next.depth);
      if (gl.checkFramebufferStatus(gl.FRAMEBUFFER) !== gl.FRAMEBUFFER_COMPLETE) throw new Error('Incomplete full-map PIP framebuffer');
    } catch (error) {
      destroy(next);
      throw error;
    }
    if (target) retired.push(target);
    target = next;
  }

  return Object.freeze({
    // drawWorld renders the existing world scene for this view without
    // advancing simulation, audio, interpolation clocks or the main camera.
    render({camera, width, height, heightAt, drawWorld, clearColor = [.012, .03, .05, 1]}) {
      if (disposed || rendering) throw new Error('Full-map PIP renderer unavailable');
      const limit = Math.min(gl.getParameter(gl.MAX_TEXTURE_SIZE), gl.getParameter(gl.MAX_RENDERBUFFER_SIZE));
      if (![width, height].every(value => Number.isInteger(value) && value > 0 && value <= limit) ||
          ![camera?.x, camera?.y, camera?.heading, camera?.nativeFraction].every(Number.isFinite) ||
          !clearColor.every(Number.isFinite) || clearColor.length !== 4 || typeof drawWorld !== 'function') {
        throw new RangeError('Invalid full-map PIP pass');
      }
      const view = updateOriginalCamera({...camera}, {vp: new Float32Array(16)}, width, height, heightAt, ORIGINAL_PIP_CAMERA_PROFILE);
      const before = saveSceneGLState(gl);
      rendering = true;
      try {
        for (let index = retired.length - 1; index >= 0; index--) {
          const row = retired[index];
          if (before.draw !== row.framebuffer && before.read !== row.framebuffer && before.renderbuffer !== row.depth &&
              !before.textures.some(([texture]) => texture === row.texture)) {
            destroy(row);
            retired.splice(index, 1);
          }
        }
        allocate(width, height);
        for (let unit = 0; unit < before.textures.length; unit++) {
          if (before.textures[unit][0] === target.texture) {
            gl.activeTexture(gl.TEXTURE0 + unit);
            gl.bindTexture(gl.TEXTURE_2D, null);
          }
        }
        gl.activeTexture(gl.TEXTURE0);
        gl.bindTexture(gl.TEXTURE_2D, null);
        gl.bindFramebuffer(gl.FRAMEBUFFER, target.framebuffer);
        gl.viewport(0, 0, width, height);
        for (const cap of [gl.SCISSOR_TEST, gl.STENCIL_TEST, gl.BLEND, gl.CULL_FACE]) gl.disable(cap);
        gl.enable(gl.DEPTH_TEST);
        gl.colorMask(true, true, true, true);
        gl.depthMask(true);
        gl.depthFunc(gl.LESS);
        gl.stencilMask(255);
        gl.clearColor(...clearColor);
        gl.clearDepth(1);
        gl.clearStencil(0);
        gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT | gl.STENCIL_BUFFER_BIT);
        const output = drawWorld({...view, width, height, capture: true});
        if (output && typeof output.then === 'function') throw new TypeError('Full-map PIP drawWorld must be synchronous');
        return {texture: target.texture, view, width, height};
      } finally {
        rendering = false;
        restoreSceneGLState(gl, before);
      }
    },
    dispose() {
      if (rendering) throw new Error('Cannot dispose during full-map PIP capture');
      if (target) destroy(target);
      for (const row of retired) destroy(row);
      target = null;
      retired.length = 0;
      disposed = true;
    },
  });
}
