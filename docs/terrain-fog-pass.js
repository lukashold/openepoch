import {createNativeTerrainFogAlpha,nativeTerrainFogSampler} from './native-fog-alpha.js?v=terrain-presentation227';

const vertex = `#version 300 es
layout(location=0) in vec3 aPosition;
uniform mat4 uVP;
uniform float uWorldHeight;
uniform bool uSourceYForward;
uniform float uTextureSize;
out vec2 vFog;
out float vHeight;
void main() {
  vFog = vec2(aPosition.x, uSourceYForward ? aPosition.z : uWorldHeight - aPosition.z) / uTextureSize;
  vHeight = aPosition.y;
  gl_Position = uVP * vec4(aPosition,1.0);
}`;

// The source material contributes black RGB at all eight levels. Its own
// alpha is ignored. The one-level dynamic visibility texture supplies alpha.
const fragment = `#version 300 es
precision highp float;
uniform sampler2D uAlpha;
uniform bool uLandOnly;
uniform float uMinHeight;
in vec2 vFog;
in float vHeight;
out vec4 outColor;
void main() {
  float alpha = texture(uAlpha,vFog).r;
  if (uLandOnly && vHeight < uMinHeight) discard;
  outColor = vec4(0.0,0.0,0.0,alpha);
}`;

function makeProgram(gl) {
  const program=gl.createProgram();
  if (!program) throw Error('Cannot allocate terrain fog program');
  try {
    for (const [type,source] of [[gl.VERTEX_SHADER,vertex],[gl.FRAGMENT_SHADER,fragment]]) {
      const shader=gl.createShader(type);
      if (!shader) throw Error('Cannot allocate terrain fog shader');
      try {
        gl.shaderSource(shader,source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader,gl.COMPILE_STATUS)) throw Error(gl.getShaderInfoLog(shader));
        gl.attachShader(program,shader);
      } finally {
        gl.deleteShader(shader);
      }
    }
    gl.linkProgram(program);
    if (!gl.getProgramParameter(program,gl.LINK_STATUS)) throw Error(gl.getProgramInfoLog(program));
    return program;
  } catch (error) {
    gl.deleteProgram(program);
    throw error;
  }
}

export function createTerrainFogPass(gl,{width,height,stride=width,sourceYForward=false}) {
  const field=createNativeTerrainFogAlpha({width,height,stride});
  const program=makeProgram(gl);
  const texture=gl.createTexture();
  if (!texture) {
    gl.deleteProgram(program);
    throw Error('Cannot allocate terrain fog texture');
  }
  const uniforms=Object.fromEntries(['uVP','uWorldHeight','uTextureSize','uAlpha','uLandOnly','uMinHeight','uSourceYForward']
    .map(name=>[name,gl.getUniformLocation(program,name)]));
  const anisotropic=gl.getExtension('EXT_texture_filter_anisotropic');
  const maxAnisotropy=anisotropic?gl.getParameter(anisotropic.MAX_TEXTURE_MAX_ANISOTROPY_EXT):1;
  let quality, disposed=false;

  function bindTexture(fn) {
    const unit=gl.getParameter(gl.ACTIVE_TEXTURE);
    gl.activeTexture(gl.TEXTURE0);
    const previous=gl.getParameter(gl.TEXTURE_BINDING_2D);
    gl.bindTexture(gl.TEXTURE_2D,texture);
    try {
      return fn();
    } finally {
      gl.bindTexture(gl.TEXTURE_2D,previous);
      gl.activeTexture(unit);
    }
  }

  bindTexture(()=>{
    const target=gl.TEXTURE_2D;
    gl.texStorage2D(target,1,gl.R8,field.size,field.size);
    gl.texParameteri(target,gl.TEXTURE_WRAP_S,gl.CLAMP_TO_EDGE);
    gl.texParameteri(target,gl.TEXTURE_WRAP_T,gl.CLAMP_TO_EDGE);
  });

  function upload(pixels) {
    const align=gl.getParameter(gl.UNPACK_ALIGNMENT);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT,1);
    try {
      bindTexture(()=>gl.texSubImage2D(gl.TEXTURE_2D,0,0,0,field.size,field.size,gl.RED,gl.UNSIGNED_BYTE,pixels));
    } finally {
      gl.pixelStorei(gl.UNPACK_ALIGNMENT,align);
    }
  }
  upload(field.pixels);

  function update(visibility,options) {
    if (disposed) throw Error('Terrain fog pass is disposed');
    upload(field.update(visibility,options));
  }

  function draw({vp,vao,indexCount,graphicsQuality=1,minHeight=null}) {
    if (disposed) throw Error('Terrain fog pass is disposed');
    if (!indexCount) return 0;
    const get=constant=>gl.getParameter(constant);
    const depth=get(gl.DEPTH_FUNC),mask=get(gl.DEPTH_WRITEMASK);
    const previousProgram=get(gl.CURRENT_PROGRAM),previousVao=get(gl.VERTEX_ARRAY_BINDING);
    const blend=[gl.BLEND_SRC_RGB,gl.BLEND_DST_RGB,gl.BLEND_SRC_ALPHA,gl.BLEND_DST_ALPHA].map(get);
    const equation=[gl.BLEND_EQUATION_RGB,gl.BLEND_EQUATION_ALPHA].map(get);
    const caps=[gl.DEPTH_TEST,gl.BLEND,gl.CULL_FACE,gl.POLYGON_OFFSET_FILL].map(cap=>[cap,gl.isEnabled(cap)]);
    const high=graphicsQuality>=2;
    try {
      return bindTexture(()=>{
        const target=gl.TEXTURE_2D;
        const previousSampler=get(gl.SAMPLER_BINDING);
        gl.bindSampler(0,null);
        try {
          if (high!==quality) {
            const sampler=nativeTerrainFogSampler(graphicsQuality);
            gl.texParameteri(target,gl.TEXTURE_MIN_FILTER,sampler.minification==='point'?gl.NEAREST:gl.LINEAR);
            gl.texParameteri(target,gl.TEXTURE_MAG_FILTER,gl.LINEAR);
            if (anisotropic) gl.texParameterf(target,anisotropic.TEXTURE_MAX_ANISOTROPY_EXT,
              Math.min(sampler.maxAnisotropy,maxAnisotropy));
            quality=high;
          }
          gl.enable(gl.DEPTH_TEST);
          gl.enable(gl.BLEND);
          gl.disable(gl.CULL_FACE);
          gl.disable(gl.POLYGON_OFFSET_FILL);
          gl.depthFunc(gl.LEQUAL);
          gl.depthMask(false);
          gl.blendEquation(gl.FUNC_ADD);
          gl.blendFuncSeparate(gl.SRC_ALPHA,gl.ONE_MINUS_SRC_ALPHA,gl.ZERO,gl.ONE);
          gl.useProgram(program);
          gl.bindVertexArray(vao);
          gl.uniformMatrix4fv(uniforms.uVP,false,vp);
          gl.uniform1f(uniforms.uWorldHeight,height);
          gl.uniform1i(uniforms.uSourceYForward,Number(sourceYForward));
          gl.uniform1f(uniforms.uTextureSize,field.size);
          gl.uniform1i(uniforms.uAlpha,0);
          gl.uniform1i(uniforms.uLandOnly,Number(minHeight!==null));
          gl.uniform1f(uniforms.uMinHeight,minHeight??0);
          gl.drawElements(gl.TRIANGLES,indexCount,gl.UNSIGNED_INT,0);
          return indexCount;
        } finally {
          gl.bindSampler(0,previousSampler);
        }
      });
    } finally {
      gl.useProgram(previousProgram);
      gl.bindVertexArray(previousVao);
      gl.depthFunc(depth);
      gl.depthMask(mask);
      gl.blendFuncSeparate(...blend);
      gl.blendEquationSeparate(...equation);
      for (const [cap,enabled] of caps) (enabled?gl.enable:gl.disable).call(gl,cap);
    }
  }

  function dispose() {
    if (disposed) return;
    disposed=true;
    gl.deleteTexture(texture);
    gl.deleteProgram(program);
  }

  return {update,draw,dispose,size:field.size};
}
