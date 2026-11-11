const URL_PATH = new URL('./local-original/torpedo-muzzle/anchors.bin', import.meta.url);
const DIGEST = '772a6e9711347df69a3316e9791452fc6755082cf64d3ede5d4b31a1d4ebb8a8';

export async function loadPrivateTorpedoAnchors(wasm, fetchFile = globalThis.fetch) {
  if (typeof wasm.configure_torpedo_model_anchors !== 'function' || !globalThis.crypto?.subtle) return null;
  try {
    const response = await fetchFile(URL_PATH);
    if (!response.ok) return null;
    const bytes = await response.arrayBuffer();
    if (bytes.byteLength !== 88) return null;
    const hash = await crypto.subtle.digest('SHA-256', bytes);
    const hex = Array.from(new Uint8Array(hash), word => word.toString(16).padStart(2, '0')).join('');
    return hex === DIGEST ? bytes : null;
  } catch {
    return null;
  }
}

export function applyPrivateTorpedoAnchors(wasm, bytes) {
  const pointer = wasm.load_alloc(bytes.byteLength);
  new Uint8Array(wasm.memory.buffer, pointer, bytes.byteLength).set(new Uint8Array(bytes));
  return wasm.configure_torpedo_model_anchors() === 1;
}
