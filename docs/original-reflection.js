// Source WaterManager0088A8F0/0088E210 brackets a reflection target and clip plane;
//0088DE20 clones the view and negates native Z-up camera height. This Y-up VP
// mirror and fragment clipping adapt that horizontal-plane behavior to WebGL.
// Native auxiliary projection/near-clamp, allocation and scene culling remain
// separate; see analysis/native-reflection-evidence.json. Resolution and GL state
// management below are platform adapters, not invented original engine policy.
const finiteArray=(value,length)=>value?.length===length&&Array.from(value).every(Number.isFinite);

/** Column-major VP * reflection in the horizontal world plane y=height. */
export function reflectedViewProjection(vp,height=0){
 if(!finiteArray(vp,16)||!Number.isFinite(height))throw new RangeError('Invalid reflection camera');
 const result=new Float32Array(vp);
 for(let row=0;row<4;row++){result[4+row]=-vp[4+row];result[12+row]=vp[12+row]+2*height*vp[4+row];}
 return result;
}

function saveState(gl){
 const get=key=>gl.getParameter(key),active=get(gl.ACTIVE_TEXTURE),textures=[];
 // Production passes use both 2D and array textures through unit13. Retaining
 // all fragment slots also protects callers which use a different binding map.
 for(let unit=0;unit<get(gl.MAX_TEXTURE_IMAGE_UNITS);unit++){
  gl.activeTexture(gl.TEXTURE0+unit);
  textures.push([get(gl.TEXTURE_BINDING_2D),get(gl.TEXTURE_BINDING_2D_ARRAY)]);
 }
 gl.activeTexture(active);
 return {active,textures,draw:get(gl.DRAW_FRAMEBUFFER_BINDING),read:get(gl.READ_FRAMEBUFFER_BINDING),renderbuffer:get(gl.RENDERBUFFER_BINDING),
  viewport:get(gl.VIEWPORT),scissor:get(gl.SCISSOR_BOX),clearColor:get(gl.COLOR_CLEAR_VALUE),clearDepth:get(gl.DEPTH_CLEAR_VALUE),clearStencil:get(gl.STENCIL_CLEAR_VALUE),
  colorMask:get(gl.COLOR_WRITEMASK),depthMask:get(gl.DEPTH_WRITEMASK),depthFunc:get(gl.DEPTH_FUNC),stencilMask:get(gl.STENCIL_WRITEMASK),stencilBackMask:get(gl.STENCIL_BACK_WRITEMASK),
  blend:[get(gl.BLEND_SRC_RGB),get(gl.BLEND_DST_RGB),get(gl.BLEND_SRC_ALPHA),get(gl.BLEND_DST_ALPHA)],equation:[get(gl.BLEND_EQUATION_RGB),get(gl.BLEND_EQUATION_ALPHA)],
  frontFace:get(gl.FRONT_FACE),cullFace:get(gl.CULL_FACE_MODE),program:get(gl.CURRENT_PROGRAM),vao:get(gl.VERTEX_ARRAY_BINDING),array:get(gl.ARRAY_BUFFER_BINDING),
  flags:[gl.BLEND,gl.DEPTH_TEST,gl.CULL_FACE,gl.SCISSOR_TEST,gl.STENCIL_TEST].map(cap=>[cap,gl.isEnabled(cap)])};
}
function restoreState(gl,state){
 gl.bindFramebuffer(gl.DRAW_FRAMEBUFFER,state.draw);gl.bindFramebuffer(gl.READ_FRAMEBUFFER,state.read);gl.bindRenderbuffer(gl.RENDERBUFFER,state.renderbuffer);
 gl.viewport(...state.viewport);gl.scissor(...state.scissor);gl.clearColor(...state.clearColor);gl.clearDepth(state.clearDepth);gl.clearStencil(state.clearStencil);
 gl.colorMask(...state.colorMask);gl.depthMask(state.depthMask);gl.depthFunc(state.depthFunc);gl.stencilMaskSeparate(gl.FRONT,state.stencilMask);gl.stencilMaskSeparate(gl.BACK,state.stencilBackMask);
 gl.blendFuncSeparate(...state.blend);gl.blendEquationSeparate(...state.equation);gl.frontFace(state.frontFace);gl.cullFace(state.cullFace);
 for(const [cap,enabled] of state.flags)(enabled?gl.enable:gl.disable).call(gl,cap);
 gl.useProgram(state.program);gl.bindVertexArray(state.vao);gl.bindBuffer(gl.ARRAY_BUFFER,state.array);
 for(let unit=0;unit<state.textures.length;unit++){
  gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(gl.TEXTURE_2D,state.textures[unit][0]);gl.bindTexture(gl.TEXTURE_2D_ARRAY,state.textures[unit][1]);
 }
 gl.activeTexture(state.active);
}

/**
 * Synchronous render-only helper. drawScene receives {vp,eye,clipPlane,capture:true}
 * and must draw terrain/objects with that clip plane; it must not draw water or
 * sample this pass's target. No picking/camera/simulation state is changed here.
 * render() returns the texture and projection for drawWater(). Exceptions restore
 * the caller's state and are rethrown; the incomplete capture is never published.
 */
export function createOriginalReflection(gl,{resolutionScale=1,maxSize=2048}={}){
 if(!Number.isFinite(resolutionScale)||resolutionScale<=0||resolutionScale>2||!Number.isInteger(maxSize)||maxSize<1)throw new RangeError('Invalid reflection resolution policy');
 let target=null,busy=false,disposed=false;const retired=[];
 const metrics={frames:0,resizes:0,width:0,height:0};
 const destroy=row=>{if(row){gl.deleteFramebuffer(row.framebuffer);gl.deleteRenderbuffer(row.depth);gl.deleteTexture(row.texture);}};
 function allocate(width,height){
  const limit=Math.min(maxSize,gl.getParameter(gl.MAX_TEXTURE_SIZE),gl.getParameter(gl.MAX_RENDERBUFFER_SIZE));
  const fit=Math.min(resolutionScale,limit/width,limit/height);
  width=Math.max(1,Math.round(width*fit));height=Math.max(1,Math.round(height*fit));
  if(target?.width===width&&target?.height===height)return;
  const next={width,height,texture:gl.createTexture(),framebuffer:gl.createFramebuffer(),depth:gl.createRenderbuffer()};
  try{
   gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,next.texture);
   gl.texImage2D(gl.TEXTURE_2D,0,gl.RGBA8,width,height,0,gl.RGBA,gl.UNSIGNED_BYTE,null);
   for(const key of [gl.TEXTURE_MIN_FILTER,gl.TEXTURE_MAG_FILTER])gl.texParameteri(gl.TEXTURE_2D,key,gl.LINEAR);
   for(const key of [gl.TEXTURE_WRAP_S,gl.TEXTURE_WRAP_T])gl.texParameteri(gl.TEXTURE_2D,key,gl.CLAMP_TO_EDGE);
   gl.bindFramebuffer(gl.FRAMEBUFFER,next.framebuffer);gl.framebufferTexture2D(gl.FRAMEBUFFER,gl.COLOR_ATTACHMENT0,gl.TEXTURE_2D,next.texture,0);
   gl.bindRenderbuffer(gl.RENDERBUFFER,next.depth);gl.renderbufferStorage(gl.RENDERBUFFER,gl.DEPTH24_STENCIL8,width,height);
   gl.framebufferRenderbuffer(gl.FRAMEBUFFER,gl.DEPTH_STENCIL_ATTACHMENT,gl.RENDERBUFFER,next.depth);
   if(gl.checkFramebufferStatus(gl.FRAMEBUFFER)!==gl.FRAMEBUFFER_COMPLETE)throw new Error('Incomplete water reflection framebuffer');
  }catch(error){destroy(next);throw error;}
  if(target)retired.push(target);target=next;metrics.resizes++;metrics.width=width;metrics.height=height;
 }
 return {enabled:true,metrics,policy:'source-reflection-input-planar-webgl-adapter',
  render({vp,eye,width,height,waterHeight=0,clearColor=[.52,.69,.82,1],drawScene}){
   if(disposed)throw new Error('Water reflection renderer is disposed');
   if(busy)throw new Error('Recursive water reflection capture');
   if(!finiteArray(eye,3)||!finiteArray(clearColor,4)||!Number.isFinite(width)||width<1||!Number.isFinite(height)||height<1||typeof drawScene!=='function')throw new RangeError('Invalid water reflection pass');
   const reflectionVP=reflectedViewProjection(vp,waterHeight),reflectedEye=[eye[0],2*waterHeight-eye[1],eye[2]],clipPlane=[0,1,0,-waterHeight];
   const state=saveState(gl);busy=true;
   try{
    for(let i=retired.length-1;i>=0;i--)if(!state.textures.some(pair=>pair[0]===retired[i].texture)&&state.draw!==retired[i].framebuffer&&state.read!==retired[i].framebuffer){destroy(retired[i]);retired.splice(i,1);}
    allocate(width,height);
    // The previous frame's reflection may still be bound by the water pass.
    // Unbind it on every unit before attaching it as this frame's draw target.
    for(let unit=0;unit<state.textures.length;unit++)if(state.textures[unit][0]===target.texture){gl.activeTexture(gl.TEXTURE0+unit);gl.bindTexture(gl.TEXTURE_2D,null);}
    gl.activeTexture(gl.TEXTURE0);gl.bindTexture(gl.TEXTURE_2D,null);
    gl.bindFramebuffer(gl.FRAMEBUFFER,target.framebuffer);gl.viewport(0,0,target.width,target.height);
    gl.disable(gl.SCISSOR_TEST);gl.disable(gl.STENCIL_TEST);gl.disable(gl.BLEND);gl.disable(gl.CULL_FACE);gl.enable(gl.DEPTH_TEST);
    gl.depthMask(true);gl.depthFunc(gl.LESS);gl.colorMask(true,true,true,true);gl.stencilMask(255);
    gl.clearColor(...clearColor);gl.clearDepth(1);gl.clearStencil(0);gl.clear(gl.COLOR_BUFFER_BIT|gl.DEPTH_BUFFER_BIT|gl.STENCIL_BUFFER_BIT);
    const value=drawScene({vp:reflectionVP,eye:reflectedEye,clipPlane,capture:true,width:target.width,height:target.height});
    if(value&&typeof value.then==='function')throw new TypeError('Reflection drawScene must be synchronous');
    metrics.frames++;
    return {texture:target.texture,vp:reflectionVP,eye:reflectedEye,width:target.width,height:target.height,waterHeight};
   }finally{busy=false;restoreState(gl,state);}
  },
  dispose(){if(busy)throw new Error('Cannot dispose during reflection capture');destroy(target);for(const row of retired)destroy(row);retired.length=0;target=null;disposed=true;},
 };
}
