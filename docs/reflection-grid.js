const f32 = Math.fround;
const GRID_PLANE_PADDING = f32(.1);
const DISTORTION_UV_SCALE = f32(.08);

// Original 008F62D0 -> 008E4530. Viewport order is left, right, top, bottom.
function gridScreenPoint(camera, screenWidth, screenHeight, pixelX, pixelY) {
  const [left,right,top,bottom] = camera.viewport;
  const normalizedX = f32((pixelX >>> 0) / screenWidth);
  const normalizedY = f32(1 - (pixelY >>> 0) / screenHeight);
  if (normalizedX < left || normalizedX > right || normalizedY < bottom || normalizedY > top) {
    throw new RangeError('Grid ray is outside its source viewport');
  }
  const [lowX,highX,highY,lowY] = camera.frustum;
  return [f32(lowX + (highX-lowX)*((normalizedX-left)/(right-left))),
    f32(lowY + (highY-lowY)*((normalizedY-bottom)/(top-bottom)))];
}

function unitDirection(direction) {
  const [dx,dy,dz] = direction;
  const length = Math.sqrt((dx*dx + dy*dy) + dz*dz);
  if (!(length > 0)) throw new RangeError('Zero-length grid ray');
  const reciprocal = 1/length;
  return direction.map(value => f32(value*reciprocal));
}

// Original perspective branch of 008E45F0.
export function perspectiveGridRay(camera, screenWidth, screenHeight, pixelX, pixelY) {
  const [x,y] = gridScreenPoint(camera,screenWidth,screenHeight,pixelX,pixelY);
  const direction = camera.forward.map((forward,axis) => f32((f32(camera.right[axis]*x)+forward)+f32(camera.up[axis]*y)));
  return {origin:camera.eye, direction:unitDirection(direction)};
}

export function orthographicGridRay(camera, screenWidth, screenHeight, pixelX, pixelY) {
  const [x,y] = gridScreenPoint(camera,screenWidth,screenHeight,pixelX,pixelY);
  const [left,right,top,bottom] = camera.frustum;
  const horizontalShift = f32(-(left+right)), verticalShift = -(top+bottom);
  const origin = camera.eye.map((value,axis) => {
    const alongRight = f32(value+f32(camera.right[axis]*x));
    const alongUp = f32(alongRight+f32(camera.up[axis]*y));
    const shiftedRight = f32(alongUp+f32(camera.right[axis]*horizontalShift));
    return f32(shiftedRight+f32(camera.up[axis]*verticalShift));
  });
  return {origin, direction:unitDirection(camera.forward)};
}

function planeThrough(normal, point) {
  return [...normal, f32(-((point[2]*normal[2]+point[1]*normal[1])+point[0]*normal[0]))];
}

function intersect({origin,direction}, [nx,ny,nz,distance]) {
  const [x,y,z] = origin;
  const [dx,dy,dz] = direction;
  const dot = (dy*ny+dx*nx)+dz*nz;
  if (dot === 0 || !Number.isFinite(dot)) throw new RangeError('Grid ray has no intersection');
  const t = (-1/dot)*(((x*nx+y*ny)+z*nz)+distance);
  return [f32(x+dx*t),f32(y+f32(dy*t)),f32(z+f32(dz*t))];
}

function gridStep(start,end,inverse) {
  return [f32((end[0]-start[0])*inverse), f32((end[1]-start[1])*inverse),
    f32(f32(end[2]-start[2])*inverse)];
}

// The direct camera supplies the world rays and viewport. The reflected
// texture and the offscreen capture projection are separate inputs to drawing.
export function createNativeReflectionGrid({camera, screenWidth, screenHeight, columns, rows,
  screenRay = null, offsets = null}) {
  if (![columns,rows,screenWidth,screenHeight].every(Number.isInteger) || columns < 2 || rows < 2 ||
      columns*rows > 65536 || screenWidth < 1 || screenHeight < 1 ||
      (offsets && offsets.length !== 2*columns*rows)) throw new RangeError('Invalid reflection grid dimensions');
  const [left,right,top,bottom] = camera.viewport;
  const width = f32((right-left)*screenWidth);
  const height = f32((top-bottom)*screenHeight);
  const x0 = f32(left*screenWidth);
  const x1 = f32(x0+width);
  const screenBottom = (1-bottom)*screenHeight;
  const y0 = f32(screenBottom-height);
  const y1 = f32(screenBottom);
  const cast = camera.orthographic ? orthographicGridRay : perspectiveGridRay;
  const rayAt = (x,y) => {
    const pixelX = Math.trunc(x), pixelY = Math.trunc(y);
    return screenRay ? screenRay(pixelX,pixelY) : cast(camera,screenWidth,screenHeight,pixelX,pixelY);
  };
  const center = rayAt(x0+width*.5, screenBottom-height*.5);
  const depth = camera.frustum[4]+GRID_PLANE_PADDING;
  const [dx,dy,dz] = center.direction;
  const [ox,oy,oz] = center.origin;
  const near = planeThrough(center.direction,[f32(ox+f32(dx*depth)),f32(oy+f32(dy*depth)),f32(oz+dz*depth)]);
  const corner = intersect(rayAt(x0,y0),near);
  const rightCorner = intersect(rayAt(x1,y0),near);
  intersect(rayAt(x1,y1),near);
  const bottomCorner = intersect(rayAt(x0,y1),near);
  const uStep = f32(1/(columns-1)), vStep = f32(1/(rows-1));
  const columnStep = gridStep(corner,rightCorner,uStep);
  const rowStep = gridStep(corner,bottomCorner,vStep);
  const screenXStep = f32(width/(columns-1));
  const screenYStep = f32(height/(rows-1));
  const waterPlane = [0,0,1,-0];
  const vertices = new Float32Array(columns*rows*7);
  const indices = new Uint16Array((columns-1)*(rows-1)*6);
  let screenY = f32((1-top)*screenHeight), v = 0, at = 0;
  let [rowX,rowY,rowZ] = corner;
  for (let row = 0; row < rows; row++) {
    let screenX = x0, u = 1;
    let x = rowX, y = rowY, z = rowZ;
    for (let column = 0; column < columns; column++) {
      const index = row*columns+column;
      const hit = intersect(rayAt(screenX,screenY),waterPlane);
      vertices.set([x,y,z,u+(offsets?.[2*index]??0),v+(offsets?.[2*index+1]??0),
        hit[0]*DISTORTION_UV_SCALE,hit[1]*DISTORTION_UV_SCALE],index*7);
      x=f32(x+columnStep[0]);y=f32(y+columnStep[1]);z=f32(z+columnStep[2]);
      screenX=f32(screenX+screenXStep);u=f32(u-uStep);
      if (row < rows-1 && column < columns-1) {
        indices.set([index,index+1,index+columns+1,index,index+columns+1,index+columns],at);
        at+=6;
      }
    }
    rowX=f32(rowX+rowStep[0]);rowY=f32(rowY+rowStep[1]);rowZ=f32(rowZ+rowStep[2]);
    screenY=f32(screenY+screenYStep);v=f32(v+vStep);
  }
  return {vertices,indices,columns,rows};
}
