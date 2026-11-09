const missingChild = 0x7fffffff;
const blockStride = 25;

export function partitionTerrain(mapWidth, mapHeight) {
  const blocks = [];
  for (let y = 0; y < mapHeight - 1; y += blockStride) {
    for (let x = 0; x < mapWidth - 1; x += blockStride) {
      blocks.push({ x, y, width: Math.min(blockStride, mapWidth - 1 - x),
        height: Math.min(blockStride, mapHeight - 1 - y) });
    }
  }
  return blocks;
}

export function buildBlockTree({ x, y, width, height }, leafBounds) {
  const size = 2 ** Math.ceil(Math.log2(Math.max(width, height)));
  const nodes = [];
  function build(col, row, extent) {
    if (col >= width || row >= height) return missingChild;
    if (extent === 1) return ~(row * width + col);
    const key = nodes.length;
    const node = { children: [], low: Infinity, high: -Infinity };
    nodes.push(node);
    const half = extent / 2;
    for (let child = 0; child < 4; ++child) {
      const childKey = build(col + (child & 1) * half, row + (child >> 1) * half, half);
      node.children.push(childKey);
      if (childKey === missingChild) continue;
      const childBounds = childKey < 0 ? leafBounds(~childKey) : undefined;
      const low = childBounds ? childBounds[2] : nodes[childKey].low;
      const high = childBounds ? childBounds[5] : nodes[childKey].high;
      node.low = Math.min(node.low, low);
      node.high = Math.max(node.high, high);
    }
    return key;
  }
  return { x, y, size, root: build(0, 0, size), nodes };
}

function supportPass(box, planes) {
  return planes.every(([x, y, z, distance]) => {
    const support = (x * box[x > 0 ? 3 : 0] + z * box[z > 0 ? 5 : 2]) + y * box[y > 0 ? 4 : 1];
    return support - distance > 0;
  });
}

export function selectLeaves(tree, planes, leafBounds) {
  const selected = [];
  function visit(key, x, y, extent) {
    if (key === missingChild) return;
    if (key < 0) {
      const leaf = ~key;
      if (supportPass(leafBounds(leaf), planes)) selected.push(leaf);
      return;
    }
    const { children, low, high } = tree.nodes[key];
    if (!supportPass([x, y, low, x + extent, y + extent, high], planes)) return;
    const half = extent / 2;
    for (let i = 0; i < children.length; ++i) {
      visit(children[i], x + (i & 1) * half, y + (i >> 1) * half, half);
    }
  }
  visit(tree.root, tree.x, tree.y, tree.size);
  return selected;
}
