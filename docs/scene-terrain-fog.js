import {createTerrainFogPass} from './terrain-fog-pass.js?v=terrain-presentation227';

const valleyArchive='e817209ca71aecf885289289669822e5eb798bd16b922c9aef3616d0459c4ef4';
export function admitsValleyTerrainFog(world) {
  return world?.width===115&&world.height===71&&world.backing===256&&world.sourceArchiveSha256===valleyArchive;
}

export function createSceneTerrainFog(gl,{
  width,height,stride=width,minHeight=null,sourceYForward=false,allocate=createTerrainFogPass,
}) {
  const pass=allocate(gl,{width,height,stride:sourceYForward?stride:width,sourceYForward});
  const originalRows=sourceYForward?null:new Uint8Array(width*height);
  const metrics={size:pass.size,uploads:0,mainDraws:0,pipDraws:0};
  let countdown=0;

  function refresh(visibility,renderSeconds) {
    countdown=Math.fround(countdown-Math.fround(renderSeconds));
    if(countdown>0)return false;
    if(originalRows) {
      for(let row=0;row<height;++row) {
        const from=(height-1-row)*stride;
        originalRows.set(visibility.subarray(from,from+width),row*width);
      }
    }
    pass.update(originalRows??visibility);
    countdown=1;
    metrics.uploads++;
    return true;
  }

  function draw(camera,vp,vao,graphicsQuality,indexCount=width*height*6) {
    const count=pass.draw({vp,vao,indexCount,graphicsQuality,minHeight});
    if(count)metrics[camera==='pip'?'pipDraws':'mainDraws']++;
    return count;
  }

  function reset() {
    countdown=0;
    metrics.uploads=metrics.mainDraws=metrics.pipDraws=0;
  }

  return {refresh,draw,reset,minHeight,metrics};
}
