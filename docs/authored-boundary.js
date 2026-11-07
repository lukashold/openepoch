const CARPET_WIDTH = 50;
const MINIMUM_HEIGHT = Math.fround(0.05);

// HorizontalBorder 7135B0 for the authenticated, cliff-free Valley map.
// The four native strips share the extruded diagonal vertex at each corner.
export function authoredValleyBoundary({width, height, elevations}) {
  if (width !== 115 || height !== 71 || elevations.length !== (width + 1) * (height + 1)) {
    throw new Error('Unexpected authenticated Valley boundary');
  }
  const vertices = [];
  const heightAt = (x, y) => Math.max(MINIMUM_HEIGHT, elevations[y * (width + 1) + x]);
  const add = (x, y, offsetX = 0, offsetY = 0) => vertices.push(x + offsetX, heightAt(x, y), y + offsetY);
  const corner = (x, y, offsetX, offsetY, exteriorX, exteriorY) => {
    const start = vertices.length / 3;
    add(x, y, offsetX, offsetY);
    add(x, y, exteriorX, exteriorY);
    add(x, y);
    return start;
  };
  const edge = (x, y, dx, dy, count, offsetX, offsetY) => {
    for (let index = 1; index <= count; index++) {
      const cornerX = x + index * dx, cornerY = y + index * dy;
      add(cornerX, cornerY, offsetX, offsetY);
      add(cornerX, cornerY);
    }
  };

  const offset = CARPET_WIDTH;
  const first = corner(0, 0, -offset, -offset, 0, -offset);
  edge(0, 0, 1, 0, width, 0, -offset);
  const second = corner(width, 0, offset, -offset, offset, 0);
  edge(width, 0, 0, 1, height, offset, 0);
  const third = corner(width, height, offset, offset, 0, offset);
  edge(width, height, -1, 0, width, 0, offset);
  const fourth = corner(0, height, -offset, offset, -offset, 0);
  edge(0, height, 0, -1, height, -offset, 0);
  const count = vertices.length / 3;
  const starts = [first, second, third, fourth], ends = [second, third, fourth, count];
  const indices = new Uint16Array(count + 4);
  let cursor = 0;
  for (const [strip, start] of starts.entries()) {
    for (let index = start; index <= ends[strip]; index++) indices[cursor++] = index % count;
  }
  return {vertices: Float32Array.from(vertices), indices, stripLengths: starts.map((start, strip) => ends[strip] - start + 1)};
}
