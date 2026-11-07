// 5207F0 keeps the orbit focus inside the native map extents and computes
// native eye Y as focusY - sin(heading)*horizontal. The generic camera's
// ground plane is X/-Y. The imported mesh keeps native Y as +Z.
export function updateAuthoredCamera(project, cam, view, width, height, heightAt, profile) {
  const sourcePlane = {...cam, y: -cam.y};
  project(sourcePlane, view, width, height, (x, z) => heightAt(x, -z), profile);
  cam.nativeFraction = sourcePlane.nativeFraction;
  cam.zoom = sourcePlane.zoom;
  cam.heading = sourcePlane.heading;
  view.eye[2] = -view.eye[2];
  view.f[2] = -view.f[2];
  view.u[2] = -view.u[2];
  view.right[2] = -view.right[2];
  view.groundRight[1] = -view.groundRight[1];
  view.groundForward[1] = -view.groundForward[1];
  // Multiply the view-projection by the inverse source-to-browser Z mapping.
  for (let index = 8; index < 12; index++) view.vp[index] = -view.vp[index];
  return view;
}
