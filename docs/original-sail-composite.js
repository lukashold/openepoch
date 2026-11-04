// Source-backed initial planes only. No substitute wind or particle animation.
const clamp = value => Math.max(0, Math.min(1, value));

// 00619FB0: initial local XY cloth normal is +/−Z; the native routine takes
// abs(dot) and does not rotate the sun into the attachment's world basis.
export function nativeInitialSailDiffuse({ambient, diffuse, sunDirection}, mode = 'normal') {
  for (const values of [ambient, diffuse, sunDirection]) {
    if (values?.length !== 3 || !values.every(Number.isFinite)) throw new TypeError('Invalid native sail lighting');
  }
  const factor = mode === 'uniform' ? 1 : Math.abs(sunDirection[2]);
  return ambient.map((value, i) => Math.fround(value + factor * diffuse[i]));
}

// FlagShader.NSF: Stage0 TOP_Modulate4x(Base, Diffuse), Alpha SelectArg1;
// Stage1 TOP_Modulate(Dark, Current) for RGB and Alpha. Stage saturation occurs
// before the dark mask. Texture apply-mode inference is never used here.
export function nativeFlagStages(base, dark, diffuse) {
  return [...base.slice(0, 3).map((value, i) => clamp(value * diffuse[i] * 4) * dark[i]), base[3] * dark[3]];
}

export function appendInitialSailParts(parent, composite) {
  if (parent.source?.sha256 !== composite.source?.sha256) throw new Error('Sail attachment belongs to a different native parent NIF');
  if (!composite.effect) return parent;
  // The common mesh exporter rounds its normalization offset metadata. Rebase
  // algebraically to that exact parent basis; never apply source tag matrices
  // again, since the composite positions already contain those transforms.
  const from=composite.normalization,to=parent.normalization;
  if(from&&to&&(!Number.isFinite(from.scale)||from.scale<=0||!Number.isFinite(to.scale)||to.scale<=0||from.axis!==to.axis))throw new Error('Incompatible original sail normalization');
  const positions=part=>from&&to?part.positions.map((v,i)=>(v/from.scale-from.offset[i%3]+to.offset[i%3])*to.scale):part.positions;
  const parts = composite.parts.map(part => ({
    name: part.name,
    positions: positions(part),
    normals: part.normals,
    uvs: part.uvs,
    indices: part.indices,
    textureUrl: part.textureUrl,
    color: [1, 1, 1, 1],
    alpha: {flags: 0x12ed, threshold: 0},
    nativeSail: {
      format: 'ee2-native-initial-sail-v1',
      shader: 'FlagShader',
      damageMaskUrls: part.damageMaskUrls,
      initialDamageMask: 0,
      sourceTag: part.sourceTag,
      pins: part.pinned,
      motion: 'initial-plane-only',
      geometryColor: [1, 1, 1, 1],
      alphaSource: '006195D0: Src6/Dest7 then flags&F3FF|1201, Threshold0 => 12ED, GREATER0 after all texture stages.',
      state: {depthTest: true, depthWrite: true, depthFunction: 'LEQUAL', blend: ['SRC_ALPHA', 'ONE_MINUS_SRC_ALPHA'], cull: false},
      sampler: {address: 'CLAMP_TO_EDGE', minification: 'LINEAR', magnification: 'LINEAR', mipmap: 'NONE'},
    },
  }));
  const combined=[...parent.parts,...parts],min=[Infinity,Infinity,Infinity],max=[-Infinity,-Infinity,-Infinity];
  for(const part of combined)for(let i=0;i<part.positions?.length;i++){const a=i%3;min[a]=Math.min(min[a],part.positions[i]);max[a]=Math.max(max[a],part.positions[i]);}
  return {...parent, parts: combined, nativeSailComposite: {
    effect: composite.effect, count: parts.length, source: composite.source,
    bounds:{min,max},boundsPolicy: 'Combined immutable parent plus appended initial sail geometry, in parent normalized coordinates.',
    limits: 'Original initial planes. Native cloth physics and dynamic damage progression remain separate.',
  }};
}

export const NATIVE_SAIL_SHADERS = Object.freeze({
  vertex: `#version 300 es
precision highp float;
layout(location=0) in vec3 aPosition;
layout(location=1) in vec2 aUV;
uniform mat4 uVP, uObject;
uniform vec3 uDiffuse;
out vec2 vUV; out vec3 vWorld; out vec3 vDiffuse;
void main(){ vec4 world=uObject*vec4(aPosition,1.); vWorld=world.xyz; vUV=aUV; vDiffuse=uDiffuse; gl_Position=uVP*world; }
`,
  fragment: `#version 300 es
precision highp float;
uniform sampler2D uBase, uDark;
uniform bool uClipEnabled; uniform vec4 uClipPlane;
in vec2 vUV; in vec3 vWorld; in vec3 vDiffuse; out vec4 outColor;
void main(){
 if(uClipEnabled && dot(vec4(vWorld,1.),uClipPlane)<0.)discard;
 vec4 base=texture(uBase,vUV), dark=texture(uDark,vUV);
 vec3 stage0=clamp(base.rgb*vDiffuse*4.,0.,1.);
 float coverage=base.a*dark.a;
 if(coverage<=0.)discard;
 outColor=vec4(stage0*dark.rgb,coverage);
}
`,
});
