const PATH = 3;
const ADD_POINT = 'ui_warplans_path_addPoint';
const FINISH = 'ui_warplans_path_finish';

function nativePoint(point) {
  if (point?.length !== 2) return null;
  const value = point.map(Math.fround);
  return value.every(Number.isFinite) ? value : null;
}

export function createOriginalWarplanPath({getColorPlayer = () => -1, playCue} = {}) {
  let draft = null;
  let activePath = null;

  function create() {
    // 0076F0E0 owns a single unsent draft in the WarplanManager widget.
    if (draft) return false;
    draft = [];
    return true;
  }

  function append(point) {
    const value = nativePoint(point);
    if (!draft || !value) return false;
    // 00772B30 and 0076C7D0 append a movable endpoint on each primary press.
    if (!activePath) {
      activePath = {type: PATH, colorPlayer: getColorPlayer(), points: [value]};
      draft.push(activePath);
    }
    activePath.points.push(value);
    playCue?.(ADD_POINT);
    return true;
  }

  function move(point) {
    const value = nativePoint(point);
    if (!activePath || !value) return false;
    activePath.points[activePath.points.length - 1] = value;
    return true;
  }

  function finish() {
    if (!draft) return false;
    // 0076C960 removes the movable endpoint. A single fixed point stays in
    // the plan but 0076C3E0 draws only paths with two or more points.
    if (activePath) {
      activePath.points.pop();
      activePath = null;
      playCue?.(FINISH);
    }
    return true;
  }

  function snapshot() {
    return {draft: draft?.map(({type,colorPlayer,points}) => ({type,colorPlayer,points:points.map(p => p.slice())})) ?? null,
      drawing: !!activePath};
  }

  return Object.freeze({create, append, move, finish, snapshot,
    hasDraft: () => !!draft, hasElements: () => !!draft?.length, drawing: () => !!activePath,
    reset: () => { draft = null; activePath = null; }});
}

export function originalWarplanPathWidths(screenWidth, minimap = false) {
  // 0076C3E0 scales both integer widths against the 1600 px source width.
  const scaled = width => Math.max(1, Math.trunc(Math.trunc(screenWidth) * width / 1600));
  return minimap ? {line:scaled(5),stroke:scaled(1)} : {line:scaled(12),stroke:scaled(4)};
}

export function visitOriginalWarplanPaths(snapshot, project, visit) {
  for (const path of snapshot.draft ?? []) {
    if (path.points.length >= 2) visit(path.points.map(([x,y]) => project(x,y)),path.colorPlayer);
  }
}
