import {createGeneratedBorderSession} from './generated-border-session.js?v=territory-valley226';
import {stockFrustumFromWebGL} from './camera-frustum.js?v=territory-valley226';

// Advance the pending geometry once, in the source camera array's order.
// An individual color pass reads only its prepared block list.
export function createTerritoryCameraMode(createRenderer, palette) {
  const passes = new Map();
  let session, worldHeight, sourceYForward, first, selection = new Map();
  const metrics = {vertices:0,indices:0,drawCalls:0,prepared:[],compiled:[],totalCompiled:0,warmed:0,suppressedReflections:0};

  function setTerrain(heights,width,height,forward = false) {
    sourceYForward = forward;
    session = createGeneratedBorderSession(heights,width,height,palette,sourceYForward);
    worldHeight = height;
    first = true;
    selection = new Map();
    for (const [id, pass] of passes) {
      pass.renderer.dispose();
      passes.delete(id);
    }
    Object.assign(metrics,{vertices:0,indices:0,drawCalls:0,prepared:[],compiled:[],totalCompiled:0,warmed:0,suppressedReflections:0});
  }

  function update(draws,topologyChanged,warmup = false) {
    if (!session) return;
    session.update(draws,first || topologyChanged);
    if (first && warmup) metrics.warmed = session.prepare(Array.from({length:session.blockCount},(_,id)=>id)).length;
    first = false;
  }

  function prepare(cameras) {
    if (!session || first) return;
    const next = new Map();
    const compiled = [];
    for (const {id,vp} of cameras) {
      const selected = session.selectBlocks(stockFrustumFromWebGL(vp,worldHeight,sourceYForward));
      next.set(id,selected);
      compiled.push(...session.prepare(selected));
    }
    selection = next;
    metrics.prepared = [...selection].map(([id,blocks])=>({id,blocks}));
    metrics.compiled = compiled;
    metrics.totalCompiled = session.metrics.compiled;
  }

  function draw(options) {
    const {terrainCamera = 'main', clipPlane} = options;
    if (!session || first) return;
    if (clipPlane) {
      metrics.suppressedReflections++;
      return;
    }
    if (!selection.has(terrainCamera)) return;
    const packet = session.packet(terrainCamera,selection.get(terrainCamera));
    let pass = passes.get(terrainCamera);
    if (packet) {
      if (!pass && packet.indices.length) {
        pass = {renderer:createRenderer(),indices:0};
        passes.set(terrainCamera,pass);
      }
      if (pass) {
        pass.renderer.upload(packet);
        pass.indices = packet.indices.length;
      }
      if (terrainCamera === 'main') {
        metrics.vertices = packet.vertices.length / 9;
        metrics.indices = packet.indices.length;
      }
    }
    if (pass?.indices) {
      pass.renderer.draw(options);
      metrics.drawCalls++;
    }
  }

  return {setTerrain,update,prepare,draw,metrics};
}
