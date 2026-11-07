// Source VisualGlow constants and the Bad child radius in the active UnitModels.
const TURN = Math.fround(2 * Math.PI);
const MIN_RADIUS = Math.fround(0.95);
const MAX_RADIUS = Math.fround(1.15);
const RADIUS = Object.freeze({0:5,1:3,3:3,4:3,5:3,6:5,7:1,8:.5,47:5,48:3,49:3,
  50:3,51:3,52:3,53:3,54:3,55:3,56:5,59:1,60:1});

export function createCaptureGlow({kothDefinition = null} = {}) {
  const hasKothDefinition = Number.isInteger(kothDefinition) && kothDefinition >= 0 && kothDefinition < 65535;
  let phase = Math.fround(1.5 * Math.PI);
  return {
    advance(seconds) {
      const updated = phase + Math.fround(seconds) / 1.5 * TURN;
      phase = Math.fround(updated % TURN);
    },
    project(actors, isActive) {
      const ratio = (Math.fround(Math.sin(phase)) + 1) * 0.5;
      const radius = Math.fround((MAX_RADIUS - MIN_RADIUS) * ratio + MIN_RADIUS);
      const alpha = Math.trunc(75 + 75 * ratio) / 255;
      const list = [];
      for (const actor of actors) {
        const scale = RADIUS[actor.def] ?? (hasKothDefinition && actor.def === kothDefinition ? 5 : 0);
        if (!scale || !isActive(actor.handle)) continue;
        const diameter = 2 * Math.fround(radius * scale);
        list.push(actor.renderX, actor.renderY, actor.renderYaw, diameter, diameter);
      }
      return {list, alpha};
    },
  };
}
