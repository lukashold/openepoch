const volumeManifest = {
  url: 'local-original/valley-shadows/ruin-volumes.json',
  sha256: 'b566fbee4704e3a5b42f08f58cef840d0dd01e025a1a702916f665d339860777',
};

const vertex = `#version 300 es
layout(location=0) in vec3 aPosition;
uniform mat4 uVolumeVP;
void main(){gl_Position=uVolumeVP*vec4(aPosition,1.0);}
`;
const fragment = `#version 300 es
precision highp float;
out vec4 outColor;
void main(){outColor=vec4(0.0);}
`;

export async function loadRuinShadowAssets(checked, index, world, lighting) {
  if (lighting?.climate !== 'Temperate' || lighting.period !== 'Afternoon' || !lighting.sourceTerrainDirection) return null;
  const manifest = JSON.parse(new TextDecoder().decode(await checked(volumeManifest)));
  const light = new Uint32Array(Float32Array.from(lighting.sourceTerrainDirection).buffer);
  if (manifest.format !== 'ee2-private-valley-static-volumes-v1' || manifest.localOnly !== true ||
      manifest.period !== lighting.period || manifest.sourceArchiveSha256 !== index.sourceArchiveSha256 ||
      manifest.world0Sha256 !== index.world0.sha256 || manifest.lightWords.some((value,i)=>value!==light[i])) {
    throw new Error('Valley ruin shadows differ from the active map or light');
  }
  let nextIndex = 0;
  for (const actor of manifest.actors) {
    const model = world.models[actor.model];
    if (model?.sourceCacheSha256 !== actor.modelSha256 || model.sha256 !== actor.meshSha256 ||
        actor.indexStart !== nextIndex || actor.indexCount <= 0 || actor.indexCount % 6 ||
        world.cases.some(scene => {
          const pose = scene.poses.find(row=>row[0]===actor.id);
          return pose?.[1] !== actor.model || actor.pose.some((value,i)=>value!==pose[i+2]);
        })) throw new Error('Valley ruin shadow actor differs from its archive');
    nextIndex += actor.indexCount;
  }
  const data = await checked(manifest.payload);
  const split = manifest.vertices * 12;
  if (nextIndex !== manifest.indices || data.byteLength !== split + manifest.indices * 2 || manifest.vertices >= 65536) {
    throw new Error('Invalid Valley ruin shadow buffers');
  }
  const positions = new Float32Array(data.buffer,data.byteOffset,manifest.vertices*3);
  const indices = new Uint16Array(data.buffer,data.byteOffset+split,manifest.indices);
  if (!positions.every(Number.isFinite) || indices.some(i=>i>=manifest.vertices)) throw new Error('Invalid Valley ruin geometry');
  return {positions, indices, actors:manifest.actors};
}

export function visibleRuinShadows(assets, poses, visibility, bounds) {
  if (!assets || !poses) return [];
  const [x0,y0,x1,y1] = bounds;
  return assets.actors.filter(actor => {
    const pose = poses.find(row=>row.id===actor.id && row.model===actor.model);
    return pose && pose.x >= x0 && pose.y >= y0 && pose.x < x1 && pose.y < y1 &&
      visibility[(pose.y | 0)*256 + (pose.x | 0)];
  });
}

export function createRuinVolumeRenderer(gl, assets, compile) {
  const program = compile(gl,vertex,fragment);
  const matrix = gl.getUniformLocation(program,'uVolumeVP');
  const vao = gl.createVertexArray();
  gl.bindVertexArray(vao);
  gl.bindBuffer(gl.ARRAY_BUFFER,gl.createBuffer());
  gl.bufferData(gl.ARRAY_BUFFER,assets.positions,gl.STATIC_DRAW);
  gl.enableVertexAttribArray(0);
  gl.vertexAttribPointer(0,3,gl.FLOAT,false,12,0);
  gl.bindBuffer(gl.ELEMENT_ARRAY_BUFFER,gl.createBuffer());
  gl.bufferData(gl.ELEMENT_ARRAY_BUFFER,assets.indices,gl.STATIC_DRAW);
  return (vp,actors) => {
    const cullEnabled = gl.isEnabled(gl.CULL_FACE);
    const cullMode = gl.getParameter(gl.CULL_FACE_MODE);
    const frontFace = gl.getParameter(gl.FRONT_FACE);
    gl.useProgram(program);
    gl.uniformMatrix4fv(matrix,false,vp);
    gl.bindVertexArray(vao);
    gl.enable(gl.CULL_FACE);
    gl.frontFace(gl.CCW);
    for (const [cull,pass] of [[gl.BACK,gl.INCR_WRAP],[gl.FRONT,gl.DECR_WRAP]]) {
      gl.cullFace(cull);
      gl.stencilOp(gl.KEEP,gl.KEEP,pass);
      for (const actor of actors) gl.drawElements(gl.TRIANGLES,actor.indexCount,gl.UNSIGNED_SHORT,actor.indexStart*2);
    }
    gl.frontFace(frontFace);
    gl.cullFace(cullMode);
    if (!cullEnabled) gl.disable(gl.CULL_FACE);
  };
}
