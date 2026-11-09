import {saveSceneGLState,restoreSceneGLState} from './original-reflection.js?v=visual-seasons2';
import {cachedReflectionTargetProbe} from './original-reflection-policy.js?v=native-water-presentation231';

/** Keep separate persistent color/depth targets for the original camera slots. */
export function createSourceReflectionTargets(gl) {
  const targets = [null,null];
  const metrics = {allocations:[0,0],captures:[0,0],probeAttempts:0};
  let disposed = false, capturing = false;

  function release(target) {
    if (!target) return;
    gl.deleteFramebuffer(target.framebuffer);
    gl.deleteRenderbuffer(target.depth);
    gl.deleteTexture(target.texture);
  }

  function allocate(width,height) {
    const limit = Math.min(gl.getParameter(gl.MAX_TEXTURE_SIZE),gl.getParameter(gl.MAX_RENDERBUFFER_SIZE));
    if (width > limit || height > limit) throw Error('Reflection target exceeds WebGL limits');
    const target = {width,height,texture:gl.createTexture(),framebuffer:gl.createFramebuffer(),depth:gl.createRenderbuffer()};
    try {
      if (!target.texture || !target.framebuffer || !target.depth) throw Error('Reflection target allocation failed');
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D,target.texture);
      gl.texStorage2D(gl.TEXTURE_2D,1,gl.RGBA8,width,height);
      gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MIN_FILTER,gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
      gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);
      gl.bindFramebuffer(gl.FRAMEBUFFER,target.framebuffer);
      gl.framebufferTexture2D(gl.FRAMEBUFFER,gl.COLOR_ATTACHMENT0,gl.TEXTURE_2D,target.texture,0);
      gl.bindRenderbuffer(gl.RENDERBUFFER,target.depth);
      gl.renderbufferStorage(gl.RENDERBUFFER,gl.DEPTH24_STENCIL8,width,height);
      gl.framebufferRenderbuffer(gl.FRAMEBUFFER,gl.DEPTH_STENCIL_ATTACHMENT,gl.RENDERBUFFER,target.depth);
      if (gl.checkFramebufferStatus(gl.FRAMEBUFFER)!==gl.FRAMEBUFFER_COMPLETE) throw Error('Incomplete reflection target');
      return target;
    } catch (error) {
      release(target);
      throw error;
    }
  }

  const probe = cachedReflectionTargetProbe((width,height)=>{
    metrics.probeAttempts++;
    const before = saveSceneGLState(gl);
    try {
      return allocate(width,height);
    } catch {
      return null;
    } finally {
      restoreSceneGLState(gl,before);
    }
  },release);

  function capture({target:spec,view,clipPlane},drawScene,clearColor=[.52,.69,.82,1]) {
    if (disposed || capturing || typeof drawScene !== 'function') throw Error('Reflection target is unavailable');
    if (!spec || (spec.id!==0 && spec.id!==1) || spec.width!==(spec.id===0?512:128) || spec.height!==spec.width) {
      throw RangeError('Wrong source reflection target slot');
    }
    const before = saveSceneGLState(gl);
    capturing = true;
    try {
      if (!targets[spec.id]) {
        targets[spec.id] = allocate(spec.width,spec.height);
        metrics.allocations[spec.id]++;
      }
      const target = targets[spec.id];
      for (const [unit,[texture]] of before.textures.entries()) {
        if (texture!==target.texture) continue;
        gl.activeTexture(gl.TEXTURE0+unit);
        gl.bindTexture(gl.TEXTURE_2D,null);
      }
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D,null);
      gl.bindFramebuffer(gl.FRAMEBUFFER,target.framebuffer);
      gl.viewport(0,0,target.width,target.height);
      for (const cap of [gl.SCISSOR_TEST,gl.STENCIL_TEST,gl.BLEND,gl.CULL_FACE]) gl.disable(cap);
      gl.enable(gl.DEPTH_TEST);
      gl.depthMask(true);
      gl.depthFunc(gl.LESS);
      gl.stencilMask(255);
      gl.colorMask(true,true,true,true);
      gl.clearColor(...clearColor);
      gl.clearDepth(1);
      gl.clearStencil(0);
      gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT|gl.STENCIL_BUFFER_BIT);
      const output = drawScene({...view,clipPlane,capture:true,width:target.width,height:target.height});
      if (output?.then) throw TypeError('Reflection draw must be synchronous');
      metrics.captures[spec.id]++;
      return {...view,texture:target.texture,width:target.width,height:target.height};
    } finally {
      capturing = false;
      restoreSceneGLState(gl,before);
    }
  }

  function sample(id) {
    if (disposed || capturing || (id!==0 && id!==1)) return null;
    if (!targets[id]) {
      const before=saveSceneGLState(gl);
      try {
        const size=id===0?512:128;
        targets[id]=allocate(size,size);
        metrics.allocations[id]++;
      } finally {
        restoreSceneGLState(gl,before);
      }
    }
    return targets[id];
  }

  function reset() {
    if (capturing) throw Error('Cannot reset an active reflection target');
    for (const target of targets) release(target);
    targets.fill(null);
  }

  function dispose() {
    reset();
    disposed = true;
  }

  return {targetSupported:()=>!disposed&&probe(),capture,sample,reset,dispose,metrics};
}
