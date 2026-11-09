import {admitsReflectionActorWater} from './reflection-actors.js?v=house-reflection233';

export const HOUSE_REFLECTION_CATALOG = Object.freeze({
  url: 'local-original/house-reflection-bounds/catalog.json',
  sha256: 'c94c8645daed46df581529cdd095580b8a64c6a17038f23fa1347a32f810617d',
});

export function bindHouseReflectionBound(catalog, modelUrl, binding, model) {
  const source = catalog?.model;
  const level = binding?.levels?.[0];
  if (catalog?.format!=='ee2-static-house-reflection-bounds-v1' || !source || source.level!==0 ||
      source.url!==modelUrl || level?.url!==modelUrl || binding.sourceFamily!=='House' ||
      binding.sourceId!==1 || source.payloadSha256!==level.sha256 ||
      source.sourceSha256!==binding.sourceSha256 || source.sourceSha256!==model?.source?.sha256 ||
      !Number.isFinite(source.sourceScale) || source.sourceScale<=0 || source.sourceScale!==binding.sourceScale ||
      !Array.isArray(source.parts) || !Array.isArray(level.parts) || source.parts.length!==level.parts.length ||
      source.parts.some((id,index)=>id!==level.parts[index]) || model.animationSources?.length ||
      !Array.isArray(model.parts) || model.parts.length!==source.parts.length || model.parts.some((part,index)=>part.sourceNode!==source.parts[index] ||
        part.skinned!==false || Object.keys(part.animations??{}).length || part.clipVisibility) ||
      !Array.isArray(source.localSphere) || source.localSphere.length!==4 ||
      !source.localSphere.every(Number.isFinite) || source.localSphere[3]<=0) return null;
  return Object.freeze({scale:source.sourceScale,sphere:Object.freeze([...source.localSphere])});
}

// Specialization of 008F6D80 to the completed building's upright renderer pose.
// The source plane uses a forward map row, opposite to portable world depth.
export function placedHouseReflectionSphere(bound, actor, mapHeight, yawOffset) {
  const f32 = Math.fround;
  const yaw = f32(actor.renderYaw+yawOffset), cos = f32(Math.cos(yaw)), sin = f32(Math.sin(yaw));
  const [x,y,z,radius] = bound.sphere, scale=bound.scale;
  const rotatedX = f32(y*sin + x*cos), rotatedY = f32(-x*sin + y*cos);
  return [f32(rotatedX*scale+actor.renderX),f32(rotatedY*scale+f32(mapHeight-actor.renderY)),
    f32(f32(z*scale)+actor.renderAltitude),f32(radius*scale)];
}

// Capture references the body list admitted by the previous direct frame.
// Packet bit 0 selects the source view independently of the target slot.
export function houseReflectionPass(packet, previous, mapHeight) {
  if (!(packet.flags&2) || !previous) return null;
  return {mapHeight, localEye:[...packet.camera.eye],
    visibleBodies:packet.flags&1?previous.pipBodies:previous.mainBodies, tested:0, culled:0};
}

/** True for every fallback case. Refine only the completed, authenticated static House. */
export function admitsHouseReflection(pass, variant, actor, lodMode) {
  const bound = variant.houseReflectionBound;
  if (!pass || !bound || lodMode!=='resource' || variant.houseLodKey!=='0' ||
      !actor || actor.def!==1 || actor.built!==1 || actor.hpRatio<=0 || (actor.renderFlags&24)) return true;
  ++pass.tested;
  const sphere = placedHouseReflectionSphere(bound, actor, pass.mapHeight, variant.yawOffset);
  if (admitsReflectionActorWater(pass.localEye, sphere, pass.visibleBodies)) return true;
  ++pass.culled;
  return false;
}
