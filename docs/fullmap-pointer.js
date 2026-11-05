// Installed UP 0075C3B0 writes both normalized coordinates as floats,
// then maps them onto the inclusive terrain endpoints.
export function fullmapWorldPoint(screenX, screenY, viewport, mapBounds) {
  const {left, top, right, bottom} = viewport;
  const {minX, minY, maxX, maxY} = mapBounds;
  const x = Math.fround(screenX), y = Math.fround(screenY);
  if (!(left <= x && x <= right && top <= y && y <= bottom)) return null;
  if (!(left < right && top < bottom && minX <= maxX && minY <= maxY)) return null;
  const nx = Math.fround((x - left) / (right - left));
  const ny = Math.fround(1 - (y - top) / (bottom - top));
  return [Math.fround(minX + nx * (maxX - minX)), Math.fround(minY + ny * (maxY - minY))];
}

// The installed 0075C2E0 stores normalized X before multiplying by the
// viewport width. Y retains its intermediate precision through the multiply.
export function fullmapScreenPoint(worldX, worldY, viewport, mapBounds) {
  const {left, top, right, bottom} = viewport;
  const {minX, minY, maxX, maxY} = mapBounds;
  const x = Math.fround(worldX), y = Math.fround(worldY);
  const nx = Math.fround((x-minX)/(maxX-minX));
  const ny = 1-(y-minY)/(maxY-minY);
  return [Math.fround(left+nx*(right-left)),Math.fround(top+ny*(bottom-top))];
}

// The loop resolves a requested pane before the live tab-availability test.
export function fullmapLaunchTarget(current, argument) {
  if (typeof argument !== 'string') return current;
  switch (argument.toLowerCase()) {
    case 'citizenmanager': return 0;
    case 'warplans':
    case 'warplanscreate': return 1;
    case 'objectives': return 2;
    case 'options': return 3;
    default: return current;
  }
}
