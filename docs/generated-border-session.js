import {unitTerrainDecalPlanes} from './unit-terrain-decal.js?v=territory-geometry222';
import {partitionTerrain} from './block-quadtree.js?v=territory-geometry222';
import {createStockBorderOrder} from './stock-border-order.js?v=territory-valley226';
import {packBorderGeometry} from './border-geometry-packet.js?v=territory-valley226';

const drawStride = 8;
const stableFields = [0,1,2,3,7];

export function createGeneratedBorderSession(heights, width, height, palette, sourceYForward = false) {
  const mapWidth = width+1, mapHeight = height+1;
  if (!(heights instanceof Float32Array) || heights.length !== mapWidth*mapHeight || !(palette instanceof Float32Array)) {
    throw new RangeError('Invalid generated territory geometry inputs');
  }
  let source = heights;
  if (!sourceYForward) {
    source = new Float32Array(heights.length);
    for (let y = 0; y < mapHeight; y++) {
      const row = height-y;
      source.set(heights.subarray(row*mapWidth,(row+1)*mapWidth),y*mapWidth);
    }
  }
  // The unit decal output reads clipped positions and UV only. Its original
  // terrain normal interpolation has no effect on vertices, indices or tint.
  const order = createStockBorderOrder({mapWidth,mapHeight,heights:source,normalWords:new Int16Array(2*source.length)});
  const blockCount = partitionTerrain(mapWidth,mapHeight).length;
  const compiled = Array.from({length:blockCount},()=>[]);
  const pending = new Set();
  let records = new Float32Array(), styles = [], revision = 0;
  const packets = new Map();
  const metrics = {registrations:0, topologies:0, compiled:0, newVertexBytes:0, packets:0};

  function update(draws, rebuild = false) {
    if (!(draws instanceof Float32Array) || draws.length % drawStride !== 0) {
      throw new RangeError('Invalid territory geometry records');
    }
    if (!rebuild && (draws.length !== records.length ||
        !styles.every((_,id)=>stableFields.every(field=>draws[id*drawStride+field]===records[id*drawStride+field])))) {
      throw new RangeError('Territory topology needs a replacement event');
    }
    if (rebuild) {
      pending.clear();
      for (const [id,style] of styles.entries()) {
        order.remove(id);
        for (const index of style.blocks) compiled[index] = [];
      }
      styles = [];
      for (let offset = 0; offset < draws.length; offset += drawStride) {
        const [x,z,cosine,sine] = draws.subarray(offset,offset+4);
        const planes = unitTerrainDecalPlanes([x,sourceYForward ? z : height-z,cosine,sine]);
        const id = offset/drawStride;
        const blocks = order.register(id,planes);
        for (const index of blocks) pending.add(index);
        styles.push({planes,blocks});
      }
      metrics.registrations += styles.length;
      metrics.topologies++;
      revision++;
    }
    for (const [id,style] of styles.entries()) {
      const offset = id*drawStride, layer = draws[offset+4];
      const owner = draws[offset+6] === 0 ? 0 : draws[offset+5];
      if (style.layer === layer && style.owner === owner) continue;
      style.layer = layer;
      style.owner = owner;
      style.tint = palette.subarray(owner*3,owner*3+3);
      revision++;
    }
    records = draws.slice();
  }

  function prepare(blocks) {
    const newlyCompiled = [];
    for (const id of blocks) {
      if (!pending.has(id)) continue;
      const result = order.compileBlock(id);
      pending.delete(id);
      compiled[id] = result.draws;
      metrics.compiled++;
      metrics.newVertexBytes += result.newVertexBytes.reduce((total,bytes)=>total+bytes,0);
      newlyCompiled.push(id);
      revision++;
    }
    return newlyCompiled;
  }

  function packet(camera, blocks) {
    const previous = packets.get(camera);
    if (previous?.revision === revision && previous.blocks.length === blocks.length &&
        blocks.every((id,index)=>id===previous.blocks[index])) return null;
    const draws = blocks.flatMap(id=>compiled[id]);
    const result = packBorderGeometry(draws,id=>styles[id],height,sourceYForward);
    packets.set(camera,{revision,blocks:[...blocks]});
    metrics.packets++;
    return result;
  }

  return {update,prepare,packet,blockCount,metrics,selectBlocks:order.selectBlocks};
}
