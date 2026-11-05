const sources = {
  base: new URL('./local-original/shell-video/fe-base.mp4', import.meta.url),
  up15: new URL('./local-original/shell-video/fe-up15.mp4', import.meta.url),
};

export function mountOriginalShellVideo(video, profile, {
  page = document,
  reducedMotion = matchMedia('(prefers-reduced-motion: reduce)'),
} = {}) {
  const source = Object.hasOwn(sources, profile) ? sources[profile] : null;
  if (!source) return () => {};
  let inMenu = true;
  let failed = false;
  const canPlay = () => inMenu && !page.hidden && !reducedMotion.matches && !failed;
  function sync() {
    if (!canPlay()) {
      video.pause();
      if (reducedMotion.matches) video.hidden = true;
      return;
    }
    if (!video.getAttribute('src')) video.src = source.href;
    if (video.paused) video.play().catch(() => { /* A browser may wait for a gesture. */ });
  }
  video.addEventListener('playing', () => {
    if (canPlay()) video.hidden = false;
  });
  video.addEventListener('error', () => {
    failed = true;
    video.hidden = true;
  });
  page.addEventListener('visibilitychange', sync);
  page.addEventListener('pointerdown', sync);
  page.addEventListener('keydown', sync);
  reducedMotion.addEventListener('change', sync);
  sync();
  return visible => { inMenu = visible; sync(); };
}
