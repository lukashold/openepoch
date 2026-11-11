import {saveSceneGLState,restoreSceneGLState} from './original-reflection.js?v=visual-seasons2';

const vertex = `#version 300 es
layout(location=0) in vec3 aSourcePosition;
layout(location=1) in vec2 aReflectionUV;
layout(location=2) in vec2 aDistortionUV;
uniform mat4 uDirectVP;
uniform float uMapHeight;
uniform bool uSourceYForward;
out vec2 vReflectionUV,vDistortionUV;
void main(){
  vReflectionUV=aReflectionUV;
  vDistortionUV=aDistortionUV;
  float worldZ=uSourceYForward?aSourcePosition.y:uMapHeight-aSourcePosition.y;
  gl_Position=uDirectVP*vec4(aSourcePosition.x,aSourcePosition.z,worldZ,1.);
}`;
const fragment = `#version 300 es
precision highp float;
uniform sampler2D uWaterNormal,uSourceReflection;
uniform float uTime;
uniform bool uShaderWarp;
in vec2 vReflectionUV,vDistortionUV;
out vec4 outColor;
void main(){
  vec2 uv=vReflectionUV;
  if(uShaderWarp){
    vec2 a=vDistortionUV+fract(uTime*vec2(.01125,.02));
    vec2 b=(vDistortionUV+fract(uTime*vec2(-.01125,.01))).yx;
    uv+=(texture(uWaterNormal,a).rg+texture(uWaterNormal,b).rg-1.)*.1;
  }
  vec4 reflected=texture(uSourceReflection,vec2(uv.x,1.-uv.y));
  outColor=vec4(reflected.rgb,reflected.a*.32);
}`;

function program(gl) {
  const handle=gl.createProgram();
  try {
    for (const [kind,source] of [[gl.VERTEX_SHADER,vertex],[gl.FRAGMENT_SHADER,fragment]]) {
      const shader=gl.createShader(kind);
      try {
        gl.shaderSource(shader,source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader,gl.COMPILE_STATUS)) throw Error(gl.getShaderInfoLog(shader));
        gl.attachShader(handle,shader);
      } finally {
        gl.deleteShader(shader);
      }
    }
    gl.linkProgram(handle);
    if (!gl.getProgramParameter(handle,gl.LINK_STATUS)) throw Error(gl.getProgramInfoLog(handle));
    return handle;
  } catch(error) {
    gl.deleteProgram(handle);
    throw error;
  }
}

// Retain both face states because shadows and the previous PIP draw may have
// distinct stencil functions, operations and write masks.
export function waterStencilScope(gl,draw) {
  const read=names=>names.map(name=>gl.getParameter(gl[name]));
  const front=read(['STENCIL_FUNC','STENCIL_REF','STENCIL_VALUE_MASK','STENCIL_FAIL','STENCIL_PASS_DEPTH_FAIL','STENCIL_PASS_DEPTH_PASS','STENCIL_WRITEMASK']);
  const back=read(['STENCIL_BACK_FUNC','STENCIL_BACK_REF','STENCIL_BACK_VALUE_MASK','STENCIL_BACK_FAIL','STENCIL_BACK_PASS_DEPTH_FAIL','STENCIL_BACK_PASS_DEPTH_PASS','STENCIL_BACK_WRITEMASK']);
  const enabled=gl.isEnabled(gl.STENCIL_TEST);
  const mode=predicate=>{
    gl.enable(gl.STENCIL_TEST);
    gl.stencilMask(0xff);
    gl.stencilFunc(predicate,0x80,0xff);
    gl.stencilOp(gl.KEEP,gl.KEEP,gl.REPLACE);
  };
  try {
    return draw({mark:()=>mode(gl.ALWAYS),select:()=>mode(gl.EQUAL)});
  } finally {
    for (const [face,state] of [[gl.FRONT,front],[gl.BACK,back]]) {
      gl.stencilFuncSeparate(face,...state.slice(0,3));
      gl.stencilOpSeparate(face,...state.slice(3,6));
      gl.stencilMaskSeparate(face,state[6]);
    }
    (enabled?gl.enable:gl.disable).call(gl,gl.STENCIL_TEST);
  }
}

export function createReflectionOverlay(gl,normalTexture) {
  const handle=program(gl);
  const vao=gl.createVertexArray(),vbo=gl.createBuffer(),ibo=gl.createBuffer();
  const uniforms=Object.fromEntries(['uDirectVP','uMapHeight','uSourceYForward','uTime','uWaterNormal','uSourceReflection','uShaderWarp']
    .map(name=>[name,gl.getUniformLocation(handle,name)]));
  const previousVao=gl.getParameter(gl.VERTEX_ARRAY_BINDING),previousArray=gl.getParameter(gl.ARRAY_BUFFER_BINDING);
  try {
    gl.bindVertexArray(vao);
    gl.bindBuffer(gl.ARRAY_BUFFER,vbo);
    gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,ibo);
    for (const [slot,size,offset] of [[0,3,0],[1,2,12],[2,2,20]]) {
      gl.enableVertexAttribArray(slot);
      gl.vertexAttribPointer(slot,size,gl.FLOAT,false,28,offset);
    }
  } finally {
    gl.bindVertexArray(previousVao);
    gl.bindBuffer(gl.ARRAY_BUFFER,previousArray);
  }
  let columns, rows, indexCount=0;

  function draw({grid,vp,mapHeight,sourceYForward=false,time,texture}) {
    const before=saveSceneGLState(gl);
    const samplers=[];
    try {
      gl.useProgram(handle);
      gl.bindVertexArray(vao);
      gl.bindBuffer(gl.ARRAY_BUFFER,vbo);
      gl.bufferData(gl.ARRAY_BUFFER,grid.vertices,gl.DYNAMIC_DRAW);
      if (columns!==grid.columns || rows!==grid.rows) {
        gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,grid.indices,gl.STATIC_DRAW);
        columns=grid.columns;rows=grid.rows;indexCount=grid.indices.length;
      }
      gl.enable(gl.DEPTH_TEST);
      gl.depthMask(false);
      gl.depthFunc(gl.LEQUAL);
      gl.disable(gl.CULL_FACE);
      gl.enable(gl.BLEND);
      gl.blendEquation(gl.FUNC_ADD);
      gl.blendFunc(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA);
      for (const [unit,binding] of [[0,normalTexture],[7,texture]]) {
        gl.activeTexture(gl.TEXTURE0+unit);
        samplers.push([unit,gl.getParameter(gl.SAMPLER_BINDING)]);
        gl.bindSampler(unit,null);
        gl.bindTexture(gl.TEXTURE_2D,binding);
      }
      gl.uniform1i(uniforms.uWaterNormal,0);
      gl.uniform1i(uniforms.uSourceReflection,7);
      gl.uniformMatrix4fv(uniforms.uDirectVP,false,vp);
      gl.uniform1f(uniforms.uMapHeight,mapHeight);
      gl.uniform1i(uniforms.uSourceYForward,sourceYForward?1:0);
      gl.uniform1f(uniforms.uTime,time);
      gl.uniform1i(uniforms.uShaderWarp,grid.columns===14?1:0);
      gl.drawElements(gl.TRIANGLES,indexCount,gl.UNSIGNED_SHORT,0);
      return indexCount;
    } finally {
      for (const [unit,sampler] of samplers) gl.bindSampler(unit,sampler);
      restoreSceneGLState(gl,before);
    }
  }

  function dispose() {
    gl.deleteProgram(handle);
    gl.deleteVertexArray(vao);
    gl.deleteBuffer(vbo);
    gl.deleteBuffer(ibo);
  }

  return {draw,dispose};
}
