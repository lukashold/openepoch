// MusicCtrlLoopSet. Loading the record and seeing a missing active stream
// each call the same original random reset. Later tracks follow source order.
export function createMenuMusicUpdate(files, random) {
  if (!Array.isArray(files) || files.length === 0 || files.length > 65535) {
    throw new RangeError('An original menu playlist is required');
  }
  let next = 0;
  let pending = false;

  function reset() {
    next = random.word15() % files.length;
    pending = true;
  }

  function update(stream) {
    if (!stream) reset();
    if (!pending && stream.durationMs - stream.positionMs < 2000) pending = true;
    if (!pending) return null;
    const file = files[next];
    next = (next + 1) % files.length;
    pending = false;
    return {kind: 'play', file, fadeSeconds: 2};
  }

  return Object.freeze({reset, update, snapshot: () => ({next, pending})});
}
