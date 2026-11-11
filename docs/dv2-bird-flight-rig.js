import {sourcePackWeights} from './original-pack-rig.js?v=pack-rig129';

const f = Math.fround;
const duration = f(0.8333333730697632);
const paletteHeight = 24;
const paletteLayers = 64;
const sourceClips = ['walk', 'walk-alt'];
const scalar = new Float32Array(1), words = new Uint32Array(scalar.buffer);
const fword = word => { words[0] = word; return scalar[0]; };
const chop = value => {
  scalar[0] = value;
  if (Math.abs(scalar[0]) > Math.abs(value)) words[0]--;
  return scalar[0];
};
const gainCenter = fword(0x3f758559);
const gainIntercept = fword(0x3f82be63);
const gainGradient = fword(0xbf0852f1);
const estimateGain = value => gainIntercept + (value - gainCenter) * gainGradient;

export function birdFlightSeconds(elapsed, truncate = false) {
  const round = truncate ? chop : f;
  const seconds = Math.max(0, round(elapsed));
  return seconds > duration ? round(seconds % duration) : seconds;
}

function keyPair(keys, seconds, round) {
  if (keys.length === 1) return [keys[0], keys[0], 0];
  const at = Math.max(keys[0][0], Math.min(keys.at(-1)[0], seconds));
  let low = 0, high = keys.length - 1;
  while (high - low > 1) {
    const mid = (low + high) >> 1;
    if (at <= keys[mid][0]) high = mid;
    else low = mid;
  }
  const a = keys[low], b = keys[high];
  return [a, b, round((at - a[0]) / (b[0] - a[0]))];
}

function linear(keys, seconds, round) {
  const [a, b, at] = keyPair(keys, seconds, round);
  if (a === b) return a.slice(1);
  const from = 1 - at;
  if (a.length === 2) return [round(a[1]*from + b[1]*at)];
  return [
    round(a[1]*from + b[1]*at),
    round(round(a[2]*from) + round(b[2]*at)),
    round(round(a[3]*from) + round(b[3]*at)),
  ];
}

function rotation(keys, seconds, round) {
  const [a, b, at] = keyPair(keys, seconds, round);
  let [w, x, y, z] = a.slice(1);
  if (a !== b) {
    const dot = round(y*b[3] + x*b[2] + w*b[1] + z*b[4]);
    const small = at > 0.5 ? 1 - at : at;
    const k = 1 - dot * f(0.8227968811988831);
    const weight = small * (1 + f(0.5854921936988831) * k*k * (1 + (2*small - 3)*small));
    const blend = at > 0.5 ? 1 - weight : weight;
    const q = [w, x, y, z].map((value, i) => round(value + (b[i + 1] - value) * blend));
    const norm = q[1]**2 + q[2]**2 + q[3]**2 + q[0]**2, squared = round(norm);
    let reciprocal = estimateGain(norm);
    if (squared <= f(0.9152119755744934)) reciprocal *= estimateGain(squared*reciprocal*reciprocal);
    if (squared <= f(0.6521196961402893)) reciprocal *= estimateGain(squared*reciprocal*reciprocal);
    [w, x, y, z] = q.map(value => round(value * reciprocal));
  }
  const twiceZ=round(2*z), xx=round(2*x*x), xy=round(2*y*x), xz=round(twiceZ*x);
  const xw=round(2*x*w), yw=round(2*y*w), zw=round(twiceZ*w);
  const yy=2*y*y, zz=twiceZ*z, yz=twiceZ*y;
  return [1-(zz+yy),xy-zw,xz+yw,xy+zw,1-(zz+xx),yz-xw,xz-yw,yz+xw,1-(yy+xx)].map(round);
}

function transform(basis, translation, scale) {
  return Float32Array.from([...basis, ...translation, scale]);
}

function matrix(raw) {
  return transform([raw[0],raw[1],raw[2],raw[4],raw[5],raw[6],raw[8],raw[9],raw[10]], [raw[3],raw[7],raw[11]], 1);
}

function compose(a, b) {
  const out = new Float32Array(13);
  for (let row = 0; row < 3; row++) {
    const i = row * 3, x = a[i], y = a[i+1], z = a[i+2];
    for (let col = 0; col < 3; col++) out[i+col] = x*b[col] + y*b[col+3] + z*b[col+6];
    out[row+9] = a[row+9] + f(a[12] * f(x*b[9] + y*b[10] + z*b[11]));
  }
  out[12] = a[12] * b[12];
  return out;
}

function inverse(a) {
  const basis = [a[0],a[3],a[6],a[1],a[4],a[7],a[2],a[5],a[8]], scale = f(1/a[12]);
  const translation = [0,3,6].map(i => scale * f(-basis[i]*a[9] - basis[i+1]*a[10] - basis[i+2]*a[11]));
  return transform(basis, translation, scale);
}

export function prepareBirdFlightRig(source, model, modelPin) {
  const [part] = model.parts;
  const count = part?.positions.length / 3;
  if (source.format !== 'openepoch-private-bird-rig-v1' || source.modelSha256 !== modelPin
    || source.source.sha256 !== model.source.sha256 || model.parts.length !== 1 || !part.skinned
    || source.nodes[source.meshNode]?.sourceId !== part.sourceNode || source.normalization.axis !== 'x,z,y'
    || source.bones.length > paletteHeight || source.influences.length !== count
    || source.restPosition.length !== count*3 || source.restNormal.length !== count*3) {
    throw new Error('Original-Vogelmodell und Skelett passen nicht zusammen.');
  }
  const clips = new Map(sourceClips.map(name => {
    const clip = source.clips[name];
    if (clip?.duration !== duration || clip.tracks.length !== 24) throw new Error('Original-Vogelclip ist ungültig.');
    return [name, new Map(clip.tracks.map(track => {
      let previous;
      const rotation = track.rotation?.map(key => {
        const [time, ...value] = key;
        if (previous && previous.reduce((dot, component, i) => dot + component*value[i], 0) < 0) {
          for (let i=0; i<4; i++) value[i] = -value[i];
        }
        previous = value;
        return [time, ...value];
      });
      return [track.node, {...track, rotation}];
    }))];
  }));
  const nodes = source.nodes.map(node => ({
    parent: node.parent,
    rest: transform(node.restRotation, node.restTranslation, node.restScale),
  }));
  const vertices = new Float32Array(count * 9);
  for (let i = 0; i < count; i++) {
    vertices.set(source.restPosition.slice(i*3, i*3+3), i*9);
    vertices.set(source.restNormal.slice(i*3, i*3+3), i*9+3);
    vertices.set(sourcePackWeights(source.influences[i]), i*9+6);
  }
  return {
    parts: new Map([[part.sourceNode, {vertices, offset: 0}]]),
    normalization: Float32Array.from([...source.normalization.offset, source.normalization.scale]),
    nodes, clips, mesh: source.meshNode, root: source.skeletonRoot, skin: matrix(source.skinGlobal),
    bones: source.bones.map(bone => ({node: bone.node, bind: matrix(bone.bind)})),
  };
}

export function birdFlightLocal(rig, clip, i, seconds, truncate = false) {
  const track = rig.clips.get(clip)?.get(i), rest = rig.nodes[i].rest;
  const round = truncate ? chop : f;
  return track ? transform(
    track.rotation?.length ? rotation(track.rotation, seconds, round) : rest.subarray(0,9),
    track.translation?.length ? linear(track.translation, seconds, round) : rest.subarray(9,12),
    track.scale?.length ? Math.abs(linear(track.scale, seconds, round)[0]) : rest[12],
  ) : rest;
}

export function birdFlightPalette(rig, clip, seconds, truncate = false) {
  if (clip !== 'idle' && !rig.clips.has(clip)) return null;
  const world = [];
  for (const [i, node] of rig.nodes.entries()) {
    const local = birdFlightLocal(rig, clip, i, seconds, truncate);
    world.push(node.parent < 0 ? local : compose(world[node.parent], local));
  }
  const prefix = compose(world[rig.mesh], compose(rig.skin, inverse(world[rig.root])));
  const data = new Float32Array(paletteHeight * 12);
  for (const [i, bone] of rig.bones.entries()) {
    const a = compose(prefix, compose(world[bone.node], bone.bind));
    for (let row = 0; row < 3; row++) {
      const col = row * 3, sign = row === 1 ? -1 : 1;
      // The reused palette shader negates source Y. This private mesh uses X,Z,+Y.
      data.set([sign*a[col]*a[12],sign*a[col+1]*a[12],sign*a[col+2]*a[12],sign*a[9+row]], i*12+row*4);
    }
  }
  return data;
}

export function createBirdFlightFrames(gl, truncate = true) {
  let texture, used = 0;
  const clips = new Map();
  return {
    begin() { clips.clear(); used = 0; },
    pose(rig, clip, elapsed) {
      const seconds = clip === 'idle' ? 0 : birdFlightSeconds(elapsed, truncate);
      const poses = clips.get(rig) ?? new Map();
      clips.set(rig, poses);
      const key = clip + ':' + seconds;
      if (poses.has(key)) return poses.get(key);
      if (used >= paletteLayers) return null;
      const data = birdFlightPalette(rig, clip, seconds, truncate);
      if (!data) return null;
      gl.activeTexture(gl.TEXTURE0 + 8);
      if (!texture) {
        texture = gl.createTexture();
        if (!texture) return null;
        gl.bindTexture(gl.TEXTURE_2D_ARRAY, texture);
        gl.texStorage3D(gl.TEXTURE_2D_ARRAY, 1, gl.RGBA32F, 3, paletteHeight, paletteLayers);
        gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
        gl.texParameteri(gl.TEXTURE_2D_ARRAY, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      } else gl.bindTexture(gl.TEXTURE_2D_ARRAY, texture);
      const layer = used++;
      gl.texSubImage3D(gl.TEXTURE_2D_ARRAY, 0, 0, 0, layer, 3, paletteHeight, 1, gl.RGBA, gl.FLOAT, data);
      const pose = {texture, page: 'bird', layer, rig};
      poses.set(key, pose);
      return pose;
    },
  };
}
