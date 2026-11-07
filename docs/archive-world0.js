import {archiveGroupsFrame} from './archive-forest.js';

export function bindValleyWorld0(world0, sourceArchiveSha256, fixtures, packets) {
  if (world0?.format !== 'ee2-private-valley-world0-v1' || world0.localOnly !== true ||
      world0.sourceArchiveSha256 !== sourceArchiveSha256 || world0.models?.length !== 12 || world0.cases?.length !== 4) {
    throw new Error('Die neutralen Valley-Objekte stimmen nicht mit der Karte überein.');
  }
  const assetMapping = world0.models.map((row, renderId) => {
    if (row.id !== 1100 + renderId || row.assetKind !== 1 || row.epoch !== 0 || !row.verified) {
      throw new Error('Ungültiges Modell für ein neutrales Valley-Objekt.');
    }
    return {renderId, kind: row.assetKind, sourceId: row.id};
  });
  const world0Cases = new Map();
  for (const [index, fixture] of fixtures.entries()) {
    const source = world0.cases.find(row => row.site === fixture.site && row.checkpoint === fixture.checkpoint);
    if (!source || source.fixtureSha256 !== fixture.sha256 || source.poses.length !== 21) throw new Error('Valley-Objektbeleg fehlt.');
    const wire = new DataView(packets[index].buffer, packets[index].byteOffset, packets[index].byteLength);
    const models = world0.models;
    const ids = new Set();
    const poses = source.poses.map(([id, model, ...pose]) => {
      const at = 64 + 40 * (id - 1);
      if (!Number.isInteger(id) || id < 1 || at + 40 > wire.byteLength || !models[model] || ids.has(id) ||
          wire.getUint32(at, true) !== id || wire.getUint32(at + 4, true) !== 0 || wire.getUint32(at + 8, true) !== 0 ||
          pose.length !== 4 || pose.some((word, i) => word !== wire.getUint32(at + 20 + 4 * i, true))) {
        throw new Error('Ungültige Position eines neutralen Valley-Objekts.');
      }
      ids.add(id);
      const [x, y, z, yaw] = pose.map((_, i) => wire.getFloat32(at + 20 + 4 * i, true));
      return {id, model, x, y, z, yaw};
    });
    world0Cases.set(1 + 2 * fixture.site + fixture.checkpoint, poses);
  }
  return {assetMapping, world0Cases};
}

export function valleyWorld0Frame(binding, variant, visibility, bounds, archivedOnly = false) {
  const groups = binding.assetMapping.map(() => []);
  const [x0, y0, x1, y1] = bounds;
  for (const {id, x, y, z, yaw, model} of binding.world0Cases.get(variant) ?? []) {
    if (archivedOnly && id > 41) continue;
    if (x < x0 || y < y0 || x >= x1 || y >= y1 || !visibility[(y | 0) * 256 + (x | 0)]) continue;
    groups[model].push(x, y, z, yaw, model, 0, 0, 0, 1, 0, 0, 0);
  }
  return archiveGroupsFrame(groups);
}
