import {originalExtractionTimerGeometry} from './original-extraction-timer.js?v=native-spy';

const vertex = `#version 300 es
layout(location=0) in vec3 aOffset;
layout(location=1) in vec4 aColor;
uniform mat4 uVP;
uniform vec3 uOrigin,uRight,uUp;
out vec4 color;
void main(){
  color=aColor;
  gl_Position=uVP*vec4(uOrigin+uRight*aOffset.x+uUp*aOffset.y,1.);
}`;
const fragment = `#version 300 es
precision highp float;
in vec4 color;
out vec4 result;
void main(){result=color;}`;

export function createSpyExtractionTimer(gl) {
  const program=gl.createProgram();
  for(const [type,source] of [[gl.VERTEX_SHADER,vertex],[gl.FRAGMENT_SHADER,fragment]]){
    const shader=gl.createShader(type);
    gl.shaderSource(shader,source);gl.compileShader(shader);
    if(!gl.getShaderParameter(shader,gl.COMPILE_STATUS)){
      const error=gl.getShaderInfoLog(shader);gl.deleteShader(shader);gl.deleteProgram(program);throw Error(error);
    }
    gl.attachShader(program,shader);gl.deleteShader(shader);
  }
  gl.linkProgram(program);
  if(!gl.getProgramParameter(program,gl.LINK_STATUS)){
    const error=gl.getProgramInfoLog(program);gl.deleteProgram(program);throw Error(error);
  }
  const vp=gl.getUniformLocation(program,'uVP'),origin=gl.getUniformLocation(program,'uOrigin');
  const right=gl.getUniformLocation(program,'uRight'),up=gl.getUniformLocation(program,'uUp');
  const vao=gl.createVertexArray(),position=gl.createBuffer(),color=gl.createBuffer(),index=gl.createBuffer();
  const priorVao=gl.getParameter(gl.VERTEX_ARRAY_BINDING),priorArray=gl.getParameter(gl.ARRAY_BUFFER_BINDING);
  gl.bindVertexArray(vao);
  gl.bindBuffer(gl.ARRAY_BUFFER,position);gl.vertexAttribPointer(0,3,gl.FLOAT,false,0,0);gl.enableVertexAttribArray(0);
  gl.bindBuffer(gl.ARRAY_BUFFER,color);gl.vertexAttribPointer(1,4,gl.FLOAT,false,0,0);gl.enableVertexAttribArray(1);
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,index);
  gl.bindVertexArray(priorVao);gl.bindBuffer(gl.ARRAY_BUFFER,priorArray);

  return {draw(view, markers, now, source){
    if(!markers.length)return 0;
    const state={program:gl.getParameter(gl.CURRENT_PROGRAM),vao:gl.getParameter(gl.VERTEX_ARRAY_BINDING),
      array:gl.getParameter(gl.ARRAY_BUFFER_BINDING),blend:gl.isEnabled(gl.BLEND),cull:gl.isEnabled(gl.CULL_FACE),
      mask:gl.getParameter(gl.DEPTH_WRITEMASK), factors:['BLEND_SRC_RGB','BLEND_DST_RGB','BLEND_SRC_ALPHA','BLEND_DST_ALPHA']
        .map(key=>gl.getParameter(gl[key]))};
    let draws=0;
    try{
      gl.enable(gl.BLEND);gl.disable(gl.CULL_FACE);gl.depthMask(false);
      gl.blendFuncSeparate(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA,gl.ONE,gl.ONE_MINUS_SRC_ALPHA);
      gl.useProgram(program);gl.bindVertexArray(vao);
      gl.uniformMatrix4fv(vp,false,view.vp);gl.uniform3fv(right,view.right);gl.uniform3fv(up,view.u);
      for(const [x,y,z,end] of markers){
        const geometry=originalExtractionTimerGeometry(end-now,15,source);
        if(!geometry)continue;
        gl.bindBuffer(gl.ARRAY_BUFFER,position);gl.bufferData(gl.ARRAY_BUFFER,geometry.positions,gl.STREAM_DRAW);
        gl.bindBuffer(gl.ARRAY_BUFFER,color);gl.bufferData(gl.ARRAY_BUFFER,geometry.colors,gl.STREAM_DRAW);
        gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,geometry.indices,gl.STREAM_DRAW);
        gl.uniform3f(origin,x,z,y);gl.drawElements(gl.TRIANGLES,geometry.indices.length,gl.UNSIGNED_SHORT,0);
        draws++;
      }
    }finally{
      gl.useProgram(state.program);gl.bindVertexArray(state.vao);gl.bindBuffer(gl.ARRAY_BUFFER,state.array);
      gl.blendFuncSeparate(...state.factors);gl.depthMask(state.mask);
      if(!state.blend)gl.disable(gl.BLEND);
      if(state.cull)gl.enable(gl.CULL_FACE);
    }
    return draws;
  }};
}
