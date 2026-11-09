import {forestModelBasis,forestPose} from './forest-pose.js?v=valley-forest167';

// Forest render transport for the QA-only authored Valley start. The trees
// retain individual rows, including pairs with the same terrain cell.
const fields = ['x128', 'y128', 'z128', 'model', 'flags', 'state', 'variation'];
const f32 = Math.fround;
const INV_BYTE = f32(1 / 255); // B46FE0
const TURN = f32(2 * Math.PI); // B41414
const ZERO_SLOPE_BIAS = 2 ** -23; // B4390C

export function forestIdleYaw(variation) {
  if (!Number.isInteger(variation) || variation < 0 || variation > 255) throw new RangeError('Invalid forest variation byte');
  return f32(f32(variation * INV_BYTE) * TURN + ZERO_SLOPE_BIAS);
}

export function archiveForestBindings(forest, sourceArchiveSha256) {
  if (forest?.format !== 'ee2-private-valley-forest-v1' || forest.localOnly !== true ||
      forest.sourceArchiveSha256 !== sourceArchiveSha256 || !/^[0-9a-f]{64}$/.test(sourceArchiveSha256) ||
      forest.dimensions?.join(',') !== '115,71' || forest.rowFields?.join(',') !== fields.join(',') ||
      !Array.isArray(forest.models) || forest.models.length !== 6 || !Array.isArray(forest.rows) || forest.rows.length !== 335) {
    throw new TypeError('Forest pack differs from the active authored map');
  }
  return forest.models.map((model, renderId) => {
    if (model.id !== 1000 + renderId || model.assetKind !== 1 || model.epoch !== 0 || !model.verified ||
        !/^local-original\/valley-forest\/tre_[a-z0-9_]+\.json$/.test(model.url) || !/^[0-9a-f]{64}$/.test(model.sha256)) {
      throw new TypeError('Invalid typed forest model binding');
    }
    return {renderId, kind: model.assetKind, sourceId: model.id};
  });
}

export function archiveForestFrame(forest, visibility, view, state = null) {
  if (!(visibility instanceof Uint8Array) || visibility.length !== 256 ** 2 ||
      !Array.isArray(view) || view.length !== 4 || !view.every(Number.isFinite) || view[0] >= view[2] || view[1] >= view[3]) {
    throw new TypeError('Invalid forest display view');
  }
  const [x0, y0, x1, y1] = view;
  const [width, height] = forest.dimensions;
  const groups = forest.models.map(() => []);
  const phases = state?.phases;
  if (phases && (!(phases instanceof Uint8Array) || phases.length !== forest.rows.length)) {
    throw new TypeError('Forest phases do not match the archive');
  }
  const picks = phases ? forest.models.map(() => []) : null;
  const poses = phases ? forest.models.map(() => []) : null;
  const stumps = phases ? forest.models.map(() => []) : null;
  for (const [index, row] of forest.rows.entries()) {
    const [qx, qy, qz, model, flags, rowState, variation] = row;
    if (row.length !== fields.length || !row.every(Number.isInteger) || qx < 0 || qy < 0 || qz < 0 ||
        qx >= 128 * width || qy >= 128 * height || qz > 65535 || !groups[model] || flags !== 0x1a0 || rowState !== 0) {
      throw new RangeError('Invalid archived forest row');
    }
    const x = qx / 128, y = qy / 128;
    if (x < x0 || y < y0 || x >= x1 || y >= y1 || !visibility[(qy >>> 7) * 256 + (qx >>> 7)]) continue;
    const phase = phases?.[index] ?? 0;
    const pose = phase ? forestPose(phase, variation, state.now, state.chops[index], state.exhaustions[index]) : null;
    const yaw = pose?.yaw ?? forestIdleYaw(variation);
    if (pose?.stump) stumps[model].push(x, y, qz / 128, yaw, model, 0, 0, 0, 1, 0, 0, 0);
    if (pose && !pose.trunk) continue;
    groups[model].push(x, y, f32(qz / 128 - (pose?.sink ?? 0)), yaw, model, 0, 0, 0, 1, 0, 0, 0);
    if (picks) {
      if (pose?.tilt) poses[model].push([picks[model].length, forestModelBasis(yaw, pose.tilt)]);
      picks[model].push(phase === 2 ? 0xffff : index);
    }
  }
  const frame = archiveGroupsFrame(groups);
  if (picks) {
    frame.resourceIdentities = Uint16Array.from(picks.flat());
    frame.forestBases = new Map();
    for (const [model, group] of poses.entries()) {
      for (const [index, basis] of group) frame.forestBases.set(frame.ranges[model * 2] + index, basis);
    }
    frame.stumps = archiveGroupsFrame(stumps);
  }
  return frame;
}

export function archiveGroupsFrame(groups) {
  const ranges = new Uint32Array(groups.length * 2);
  const data = new Float32Array(groups.reduce((count, group) => count + group.length, 0));
  let at = 0;
  for (const [model, group] of groups.entries()) {
    ranges[model * 2] = at / 12;
    ranges[model * 2 + 1] = group.length / 12;
    data.set(group, at);
    at += group.length;
  }
  return {data, ranges};
}
