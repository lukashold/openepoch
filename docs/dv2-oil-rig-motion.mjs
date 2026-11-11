// Oil Rig node-space curves use the original three-second local KF clock.
function sourceTime(clip, seconds) {
 if (!Number.isFinite(seconds)) throw new RangeError('Invalid Oil Rig time');
 return Math.fround(Math.max(0, Math.min(clip.duration, seconds)));
}

function segment(keys, time) {
 if (keys.length === 1 || time <= keys[0][0]) return [keys[0], keys[0], 0];
 let high = keys.length - 1;
 if (time >= keys[high][0]) return [keys[high], keys[high], 0];
 let low = 0;
 while (high - low > 1) {
  const middle = (low + high) >>> 1;
  if (keys[middle][0] < time) low = middle;
  else high = middle;
 }
 const first = keys[low], last = keys[high];
 return [first, last, Math.fround((time - first[0]) / (last[0] - first[0]))];
}

export function oilRigQuadratic(keys, time) {
 const [first, last, alpha] = segment(keys, Math.fround(time));
 const [,value, , outgoing] = first, [,next,incoming] = last;
 const difference = next - value;
 const cubic = outgoing + incoming - 2 * difference;
 const square = 3 * difference - 2 * outgoing - incoming;
 return Math.fround(((cubic * alpha + square) * alpha + outgoing) * alpha + value);
}

export function oilRigTranslation(keys, time) {
 const [first, last, alpha] = segment(keys, Math.fround(time));
 const rest = 1 - alpha, f32 = Math.fround;
 return [f32(rest * first[1] + alpha * last[1]),
  f32(f32(rest * first[2]) + f32(alpha * last[2])),
  f32(f32(rest * first[3]) + f32(alpha * last[3]))];
}

// The engine state owns clamping/wrapping of elapsed time. Pass a local clip time here.
export function oilRigPartPose(metadata, sourceNode, clipName, seconds) {
 const clip = metadata.clips[clipName];
 if (!clip) throw new RangeError('Unknown Oil Rig animation');
 const part = metadata.parts[sourceNode], curves = clip.nodes[sourceNode];
 if (!part || !curves) return null;
 const time = sourceTime(clip, seconds);
 const angle = oilRigQuadratic(curves.xRotation, time) - part.baseAngle;
 const [,py,pz] = part.pivot;
 const target = curves.translation ? oilRigTranslation(curves.translation, time) : part.baseTranslation;
 const [x,y,z] = target.map((value, index) => value - part.baseTranslation[index]);
 const c = Math.cos(angle), s = Math.sin(angle);
 const [dx,dy,dz] = [x,z,-y].map(value => value * metadata.scale);
 return {cos:c, sin:s, translation:[dx, py - c * py + s * pz + dy, pz - s * py - c * pz + dz]};
}

export function oilRigTransformVertex(value, pose, normal = false) {
 if (!pose) return value.slice();
 const [x,y,z] = value, {cos:c, sin:s} = pose;
 const [dx,dy,dz] = normal ? [0,0,0] : pose.translation;
 return [x + dx, c * y - s * z + dy, s * y + c * z + dz];
}
