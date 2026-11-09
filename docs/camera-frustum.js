// OpenEpoch's WebGL matrix is column-major and clips depth at -w and +w.
// Map the inward clip inequalities into the source tree's X/Y/height axes.
export function stockFrustumFromWebGL(vp, worldHeight, sourceYForward = false) {
  if (vp?.length !== 16 || !Array.from(vp).every(Number.isFinite) || !Number.isFinite(worldHeight)) {
    throw new RangeError('Invalid stock terrain camera');
  }
  const planes = [];
  for (const [row,direction] of [[2,1],[2,-1],[0,1],[0,-1],[1,-1],[1,1]]) {
    const x = vp[3] + direction*vp[row];
    const z = vp[7] + direction*vp[row+4];
    const y = vp[11] + direction*vp[row+8];
    const offset = vp[15] + direction*vp[row+12];
    const scale = Math.hypot(x,y,z);
    if (!(scale > 0)) throw new RangeError('Degenerate stock terrain camera plane');
    const sourceY = sourceYForward ? y : -y;
    const shift = sourceYForward ? 0 : y*worldHeight;
    planes.push([x/scale,sourceY/scale,z/scale,-(offset+shift)/scale].map(Math.fround));
  }
  return planes;
}
