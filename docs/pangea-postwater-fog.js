export const postWaterFogFragment=`#version 300 es
precision highp float;
uniform sampler2D uMapFog;
uniform float uMapFogSide;
in vec3 vP;
out vec4 outColor;
void main(){
  float opacity=texture(uMapFog,vP.xz/uMapFogSide).r;
  if(vP.y<0.)discard;
  outColor=vec4(0.,0.,0.,opacity);
}`;

export function createPostWaterFogDraw(gl,program,vao,indexCount){
  const vp=gl.getUniformLocation(program,'uVP');
  const sampler=gl.getUniformLocation(program,'uMapFog');
  const side=gl.getUniformLocation(program,'uMapFogSide');
  const blendParams=[gl.BLEND_SRC_RGB,gl.BLEND_DST_RGB,gl.BLEND_SRC_ALPHA,gl.BLEND_DST_ALPHA];

  return (matrix,mapFog)=>{
    const blend=gl.isEnabled(gl.BLEND),depth=gl.isEnabled(gl.DEPTH_TEST);
    const blendFactors=blendParams.map(key=>gl.getParameter(key));
    const depthWrite=gl.getParameter(gl.DEPTH_WRITEMASK),depthFunc=gl.getParameter(gl.DEPTH_FUNC);
    const previousProgram=gl.getParameter(gl.CURRENT_PROGRAM),previousVao=gl.getParameter(gl.VERTEX_ARRAY_BINDING);
    const active=gl.getParameter(gl.ACTIVE_TEXTURE);
    gl.activeTexture(gl.TEXTURE7);
    const texture=gl.getParameter(gl.TEXTURE_BINDING_2D);
    try{
      // The native PostWaterRenderer keeps the scene depth and blends black.
      gl.enable(gl.BLEND);gl.enable(gl.DEPTH_TEST);gl.depthMask(false);gl.depthFunc(gl.LEQUAL);
      gl.blendFuncSeparate(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA,gl.ZERO,gl.ONE);
      gl.useProgram(program);gl.bindVertexArray(vao);gl.bindTexture(gl.TEXTURE_2D,mapFog.texture);
      gl.uniform1i(sampler,7);gl.uniform1f(side,mapFog.side);gl.uniformMatrix4fv(vp,false,matrix);
      gl.drawElements(gl.TRIANGLES,indexCount,gl.UNSIGNED_INT,0);
    }finally{
      gl.bindTexture(gl.TEXTURE_2D,texture);gl.activeTexture(active);
      gl.bindVertexArray(previousVao);gl.useProgram(previousProgram);
      gl.blendFuncSeparate(...blendFactors);gl.depthMask(depthWrite);gl.depthFunc(depthFunc);
      if(!blend)gl.disable(gl.BLEND);
      if(!depth)gl.disable(gl.DEPTH_TEST);
    }
  };
}
