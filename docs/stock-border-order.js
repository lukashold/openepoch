import {BorderBufferCache} from './border-buffer-cache.js?v=territory-geometry222';
import {clipTerrainDecal} from './clip-terrain-decal.js?v=territory-geometry222';
import {buildBlockTree, partitionTerrain, selectLeaves} from './block-quadtree.js?v=territory-geometry222';
import {assembleLeafIndices, buildStockBlock, stockLeafBounds} from './stock-terrain-mesh.js?v=territory-geometry222';

const blockExtent = 25;

function blockBox({x,y,width,height,vertices}) {
  let low = vertices[2], high = low;
  for (let i = 8; i < vertices.length; i += 6) {
    low = Math.min(low,vertices[i]);
    high = Math.max(high,vertices[i]);
  }
  return [x,y,low,x+width,y+height,Math.max(Math.fround(.05),high)];
}

// Source coordinates. Dimensions count samples, including the shared last row
// and column. Rendering a block consumes only that block's pending decals.
export function createStockBorderOrder({mapWidth, mapHeight, heights, normalWords}) {
  const samples = mapWidth * mapHeight;
  if (!Number.isSafeInteger(mapWidth) || !Number.isSafeInteger(mapHeight) || mapWidth < 2 || mapHeight < 2 ||
      !(heights instanceof Float32Array) || heights.length !== samples ||
      !(normalWords instanceof Int16Array) || normalWords.length !== 2 * samples) {
    throw new RangeError('Invalid source stock terrain dimensions');
  }

  const blocks = partitionTerrain(mapWidth, mapHeight).map(bounds => {
    const mesh = buildStockBlock({...bounds, mapWidth, heights, normalWords});
    const leafBounds = id => stockLeafBounds(mesh, id);
    const tree = buildBlockTree(bounds, leafBounds);
    return {mesh, leafBounds, tree, box:blockBox(mesh), pending:new Map(), clipped:new Map(), cache:new BorderBufferCache()};
  });
  const blockBounds = index => blocks[index].box;
  const aggregate = buildBlockTree({x:0,y:0,width:Math.ceil((mapWidth-1)/blockExtent),
    height:Math.ceil((mapHeight-1)/blockExtent)}, blockBounds);
  aggregate.size *= blockExtent;
  const registrations = new Map();

  const selectBlocks = planes => selectLeaves(aggregate,planes,blockBounds);

  function register(id, planes, selectedBlocks = selectBlocks(planes)) {
    const ids = [...selectedBlocks];
    if (registrations.has(id) || ids.length !== new Set(ids).size ||
        ids.some(index => !Number.isInteger(index) || index < 0 || index >= blocks.length)) {
      throw new RangeError('Invalid territory decal registration');
    }
    if (ids.length) registrations.set(id, ids);
    for (const index of ids) blocks[index].pending.set(id, planes);
    return ids;
  }

  function remove(id) {
    const ids = registrations.get(id);
    if (!ids) return;
    for (const index of ids) {
      const block = blocks[index];
      block.pending.delete(id);
      if (block.clipped.delete(id)) block.cache.remove(id);
    }
    registrations.delete(id);
  }

  function compileBlock(index, rebuild = false) {
    const block = blocks[index];
    if (!block) throw new RangeError('Invalid terrain block');
    const {mesh, leafBounds, tree, pending, clipped, cache} = block;
    for (const [id, planes] of pending) {
      const leaves = selectLeaves(tree, planes, leafBounds);
      if (!leaves.length) continue;
      const indices = assembleLeafIndices(mesh, leaves);
      const decal = clipTerrainDecal(mesh.vertices, indices, planes);
      if (!decal.indices.length) continue;
      cache.register(id, decal.vertices.length / 6, decal.indices.length);
      clipped.set(id, decal);
    }
    pending.clear();
    const compiled = cache.compile(rebuild);
    return {...compiled, draws:compiled.draws.map(draw => ({...draw, mesh:clipped.get(draw.id)}))};
  }

  return {register, remove, compileBlock, selectBlocks};
}
