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
