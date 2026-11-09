export const BORDER_VERTEX_STRIDE = 9;

// The inward clipping planes at indices 1 and 3 are also the original
// mirrored U and V planes for a unit border sprite with flags 0x60.
export function borderVertexUv(x,y,planes) {
  const u = planes[1], v = planes[3];
  return [Math.fround(x*u[0]+y*u[1]-u[3]),Math.fround(x*v[0]+y*v[1]-v[3])];
}

// Concatenate in source block/VB/item order. Duplicated seam pieces belong
// to separate clipped meshes. Their local indices are each rebased once.
export function packBorderGeometry(draws, styleFor, mapHeight, sourceYForward = false) {
  let vertexCount = 0, indexCount = 0;
  for (const {mesh} of draws) {
    vertexCount += mesh.vertices.length / 6;
    indexCount += mesh.indices.length;
  }
  const vertices = new Float32Array(vertexCount * BORDER_VERTEX_STRIDE);
  const indices = new Uint32Array(indexCount);
  let vertex = 0, offset = 0;
  for (const {id,mesh} of draws) {
    const {planes,layer,tint} = styleFor(id);
    const base = vertex;
    for (let i = 0; i < mesh.vertices.length; i += 6) {
      const [x,y,z] = mesh.vertices.subarray(i,i+3);
      const [u,v] = borderVertexUv(x,y,planes);
      vertices.set([x,z,sourceYForward ? y : mapHeight-y,u,v,layer,...tint],vertex*BORDER_VERTEX_STRIDE);
      vertex++;
    }
    for (const index of mesh.indices) indices[offset++] = base+index;
  }
  return {vertices,indices};
}
