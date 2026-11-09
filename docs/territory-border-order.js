// Fresh DX8 MaskOverlay buffers use first fit and prepend both pending and
// per-buffer records. Persisted buffer occupancy is outside this allocator.
const VERTICES_PER_BUFFER = 2000;

export function freshTerritoryBorderFlush(vertexCounts) {
  const buffers = [];
  const allocations = [];
  for (let index = vertexCounts.length - 1; index >= 0; index--) {
    const count = vertexCounts[index];
    if (!Number.isInteger(count) || count < 1 || count > VERTICES_PER_BUFFER)
      throw Error('Unsupported fresh territory decal vertex count');
    let buffer = buffers.findIndex(row => row.used + count <= VERTICES_PER_BUFFER);
    if (buffer === -1) {
      buffer = buffers.length;
      buffers.push({used:0,records:[]});
    }
    const row = buffers[buffer];
    allocations.push({index,buffer,count,offset:row.used});
    row.records.push(index);
    row.used += count;
  }
  const drawOrder = buffers.flatMap((row,buffer) => row.records.reverse().map(index => ({buffer,index})));
  return {allocations,drawOrder};
}
