const f32 = Math.fround;
const TURN = f32(2 * Math.PI);
const ARC_STEP = 5;
const ARROW_SIDE = 3;
const TRIANGLE_HEIGHT = f32(Math.sqrt(3) / 2);

function direction(x, y) {
  const angle = Math.atan2(y, x);
  return f32(angle < 0 ? angle + TURN : angle);
}

function rotate(x, y, angle) {
  const sin = f32(Math.sin(angle)), cos = f32(Math.cos(angle));
  return [f32(x * cos - y * sin), f32(x * sin + y * cos)];
}

function normal(angle, width) {
  return [f32(-Math.sin(angle)) * width, f32(Math.cos(angle)) * width];
}

function join(from, to, previousAngle, line) {
  const radius = line / 2;
  if (Math.hypot(to[0] - from[0], to[1] - from[1]) <= radius) return null;
  const [nx, ny] = normal(previousAngle, radius);
  const left = [f32(from[0] + nx), f32(from[1] + ny)];
  const right = [f32(from[0] - nx), f32(from[1] - ny)];
  const distanceSquared = ([x, y]) => (to[0] - x) ** 2 + (to[1] - y) ** 2;
  const clockwise = distanceSquared(right) < distanceSquared(left);
  const pivot = clockwise ? right : left;
  const dx = f32(to[0] - pivot[0]), dy = f32(to[1] - pivot[1]);
  const distance = Math.hypot(dx, dy);
  if (distance < radius) return null;

  // 00A4FB10 rotates the target ray into the tangent, choosing the side
  // whose half-width offset lies closest to the next waypoint.
  const rx = f32(from[0] - pivot[0]), ry = f32(from[1] - pivot[1]);
  const tangentAngle = Math.acos(Math.min(1, radius / f32(distance))) * (clockwise ? 1 : -1);
  let angle = direction(...rotate(dx, dy, tangentAngle)) - direction(rx, ry);
  if (angle < 0) angle += TURN;
  else if (angle >= TURN) angle -= TURN;
  if (clockwise) angle -= TURN;
  angle = f32(angle);
  const tangent = rotate(rx, ry, angle);
  const start = [f32(pivot[0] + tangent[0]), f32(pivot[1] + tangent[1])];

  // 00AADD70 divides the outside arc into approximately five-pixel edges.
  const pieces = Math.max(1, Math.round(Math.abs(angle) * line / ARC_STEP));
  const step = f32(angle / pieces), fan = [];
  let outer = [f32(from[0] + rx), f32(from[1] + ry)];
  let sweep = direction(outer[0] - pivot[0], outer[1] - pivot[1]);
  for (let piece = 0; piece < pieces; piece++) {
    const phase = sweep + step;
    sweep = f32(phase);
    const next = [f32(pivot[0] + f32(Math.cos(phase)) * line), f32(pivot[1] + f32(Math.sin(phase)) * line)];
    fan.push(angle < 0 ? [pivot, next, outer] : [pivot, outer, next]);
    outer = next;
  }
  return {start, fan};
}

export function originalWarplanGeometry(points, line, viewportHeight) {
  if (points.length < 2) return [];
  const projected = points.map(([x, y]) => [f32(x), f32(viewportHeight - y)]);
  const polygons = [];
  let previousAngle = null, arrow;
  for (let i = 1; i < projected.length; i++) {
    const from = projected[i - 1], to = projected[i];
    const bend = previousAngle === null ? {start: from, fan: []} : join(from, to, previousAngle, line);
    if (!bend) {
      previousAngle = null;
      continue;
    }
    const [x, y] = bend.start;
    const angle = direction(f32(to[0] - x), f32(to[1] - y));
    const [nx, ny] = normal(angle, line / 2);
    polygons.push([[f32(x + nx), f32(y + ny)], [f32(x - nx), f32(y - ny)],
      [f32(to[0] - nx), f32(to[1] - ny)], [f32(to[0] + nx), f32(to[1] + ny)]], ...bend.fan);
    previousAngle = angle;
    arrow = {to, angle};
  }
  if (arrow) {
    // 00AAE630 constructs the terminal equilateral triangle from the last
    // accepted segment's endpoint and direction.
    const {to: [x, y], angle} = arrow;
    const span = ARROW_SIDE * line, length = span * TRIANGLE_HEIGHT;
    const [nx, ny] = normal(angle, span / 2);
    const tip = [f32(x + f32(Math.cos(angle)) * length), f32(y + f32(Math.sin(angle)) * length)];
    polygons.push([[f32(x + nx), f32(y + ny)], [f32(x - nx), f32(y - ny)], tip]);
  }
  return polygons.map(vertices => vertices.map(([x, y]) => [x, f32(viewportHeight - y)]));
}
