// The UI and selected_actor_handle both enumerate locally owned selection in
// slot order. Query the full opaque handle; a render index is never an identity.
export function selectedCitizenContext(wasm, state) {
  if (state?.sel?.def !== 8 || !state.sel.n || wasm?.source_identity_version?.() !== 1) return undefined;
  const handle = wasm.selected_actor_handle() >>> 0;
  return Object.freeze({
    sourceIdentityVersion: 1,
    handle,
    sourceIdentity: wasm.actor_source_identity(handle) >>> 0,
    civilization: wasm.player_civilization(state.local) >>> 0,
  });
}

export const isSourceCitizenContext = (id, context) => id === 8 && context?.sourceIdentityVersion === 1;

// Presentation event emitted before the live actor slot is freed.
export function decodeCitizenDeathCue(kind) {
  const code = kind - 1024;
  if (!Number.isInteger(code) || code < 0 || code >= 450) return null;
  const context = code >>> 1;
  return {sourceIdentityVersion: 1, sourceIdentity: code & 1, civilization: context % 15, epoch: Math.floor(context / 15)};
}
