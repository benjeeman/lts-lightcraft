// LTS host extension for the pinned browser release. Uses the app's existing File-drop service and
// the LTS hook's own reads; rendering and develop commands remain in the Rust engine. No network
// paths, model credentials, filesystem paths or arbitrary commands cross this byte interface. The
// object it returns still carries the engine's own `command`. A WASM built from this fork refuses
// the community commands and opens no external page when the page has `?embedded` (Rust
// presentation gate), whoever calls it. The official 0.4.0 WASM has no such gate.
//
// v3 (R1, master plan v3 W3): in the LTS presentation (`?embedded`) a WASM built from this fork
// refuses every command id outside its LTS hook's table (`src/lts_view.rs`, `handle`), so the
// bridge sends the hook's own ids and nothing else. An import finds the photo it dropped with
// `lts.library {fileName}`, and an export reads the encoded bytes with `lts.export`, a pure output
// read: nothing is downloaded, selected or remembered (no last-export preference). Both answer the
// hook's typed envelope (`lightcraft.lts-view.v1`): a refusal names its code, and an answer that is
// not the hook's, or whose bytes are not the ones it describes, is refused here. The bridge declares
// the hook it rides (`ltsView`) next to its own version; it needs a WASM built from this fork (the
// official 0.4.0 WASM has no hook).
//
// v2: bytes cross as Uint8Array (imports still take a plain array of 0..255 integers). Every export
// passes an explicit metadata policy: `copyright` with the location removed unless the caller asks
// for `none` or `allExceptCamera` (owner decision OC-13); `all` (camera, lens and capture settings)
// is not offered. Both are checked here and always sent, and unknown parameters are refused so that
// a misspelt key cannot fall back to a default.
export const LTS_BYTE_BRIDGE_VERSION = 'lts-byte-bridge.v3';
const LTS_VIEW_VERSION = 'lightcraft.lts-view.v1';
const metadataPolicies = Object.freeze(['none', 'copyright', 'allExceptCamera']);
const exportDefaults = Object.freeze({ metadata: 'copyright', removeLocation: true });
// The %TypedArray% name getter reads an internal slot: it holds across realms (frames), a look-alike
// object cannot pass it, and it returns undefined for anything that is not a typed array.
const typedArrayName = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(Uint8Array.prototype), Symbol.toStringTag).get;
// Standard base64 (RFC 4648 section 4), padded, nothing else: what the hook writes.
const strictBase64 = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/;

export function installLtsByteBridge(api) {
  const maxInputBytes = 12 * 1024 * 1024;
  const maxOutputBytes = 32 * 1024 * 1024;
  let busy = false;
  async function exclusive(work) {
    if (busy) throw new Error('Image adjustments byte transfer is already running');
    busy = true;
    try { return await work(); } finally { busy = false; }
  }
  // The hook's answer to `id`: its typed envelope, of exactly that command; a refusal is thrown with
  // its code and message, and anything but an answer of the hook is refused.
  async function hook(id, params) {
    let reply;
    try { reply = JSON.parse(await api.command(id, JSON.stringify(params))); } catch (error) {
      if (error instanceof SyntaxError) throw new Error(`Image adjustments answered ${id} with an unexpected reply`);
      throw error;
    }
    if (!reply || typeof reply !== 'object' || reply.resultVersion !== LTS_VIEW_VERSION || reply.command !== id) {
      throw new Error(`Image adjustments answered ${id} with an unexpected reply`);
    }
    if (reply.outcome === 'not_applied') throw new Error(`Image adjustments refused ${id}: ${reply.code}: ${reply.message}`);
    if (reply.outcome !== 'ok') throw new Error(`Image adjustments answered ${id} with an unexpected reply`);
    return reply;
  }
  function checkedParams(params, allowed, kind) {
    if (!params || typeof params !== 'object' || Array.isArray(params)) throw new Error(`${kind} parameters must be an object`);
    for (const key of Object.keys(params)) {
      if (!allowed.includes(key)) throw new Error(`Unknown ${kind.toLowerCase()} parameter: ${key}`);
    }
  }
  // Returns a private copy: the caller may reuse its buffer while the import runs.
  function checkedBytes(value) {
    const typed = typedArrayName.call(value) === 'Uint8Array';
    if (!(typed || Array.isArray(value)) || !value.length || value.length > maxInputBytes) throw new Error('Invalid or excessive image bytes');
    if (typed) return new Uint8Array(value);
    const bytes = new Uint8Array(value.length);
    for (let index = 0; index < value.length; index += 1) {
      const n = value[index]; // a hole reads as undefined and is refused, never a zero byte
      if (!Number.isInteger(n) || n < 0 || n > 255) throw new Error('Invalid or excessive image bytes');
      bytes[index] = n;
    }
    return bytes;
  }
  // The bytes `lts.export` answered, in a buffer of their own, when they are the ones it describes.
  function exportedBytes(reply) {
    const text = reply.bytesBase64;
    if (typeof text !== 'string' || !text.length || text.length > Math.ceil(maxOutputBytes / 3) * 4 || !strictBase64.test(text)) {
      throw new Error('Image adjustments answered lts.export without the bytes of an image');
    }
    const binary = atob(text);
    const bytes = new Uint8Array(binary.length);
    for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
    if (!bytes.length || bytes.length > maxOutputBytes || bytes.length !== reply.byteLength) {
      throw new Error('Image adjustments answered lts.export with bytes that are not the ones it describes');
    }
    return bytes;
  }
  api.importImage = (params) => exclusive(async () => {
    checkedParams(params, ['name', 'bytes'], 'Import');
    if (typeof params.name !== 'string' || !/^[^\\/\x00-\x1f]{1,160}\.(png|jpe?g)$/i.test(params.name)) throw new Error('Invalid image name');
    const bytes = checkedBytes(params.bytes);
    const png = bytes[0] === 137 && bytes[1] === 80 && bytes[2] === 78 && bytes[3] === 71;
    const jpeg = bytes[0] === 255 && bytes[1] === 216;
    if (!png && !jpeg) throw new Error('Only PNG and JPEG image bytes are supported');
    const type = png ? 'image/png' : 'image/jpeg';
    const fileName = `lts-${crypto.randomUUID()}-${params.name}`;
    const file = new File([bytes], fileName, { type });
    const image = await createImageBitmap(file);
    const dimensions = { width: image.width, height: image.height };
    image.close();
    if (!dimensions.width || !dimensions.height || dimensions.width * dimensions.height > 16 * 1024 * 1024) throw new Error('Image exceeds the 16-megapixel session limit');
    const canvas = document.getElementById('lightcraft_canvas');
    if (!canvas || typeof DataTransfer !== 'function' || typeof DragEvent !== 'function') throw new Error('Browser File-drop import is unsupported');
    const transfer = new DataTransfer();
    transfer.items.add(file);
    canvas.dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: transfer }));
    // The page imports a dropped file on a later frame; the hook lists the library's photos of that
    // exact (unique) file name once it has.
    const deadline = performance.now() + 15000;
    while (performance.now() < deadline) {
      const reply = await hook('lts.library', { fileName });
      const photo = Array.isArray(reply.photos) ? reply.photos.find((item) => item?.fileName === fileName) : undefined;
      if (photo) return { ...photo, sourceName: params.name, transfer: 'browser-file-drop' };
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error('Image adjustments did not finish importing the image within 15 seconds');
  });
  api.exportImage = (params) => exclusive(async () => {
    checkedParams(params, ['id', 'format', 'longEdge', 'metadata', 'removeLocation'], 'Export');
    if (!Number.isSafeInteger(params.id) || params.id < 1) throw new Error('Invalid photo id');
    if (!['png', 'jpeg'].includes(params.format)) throw new Error('Only PNG and JPEG exports are supported');
    if (!Number.isSafeInteger(params.longEdge) || params.longEdge < 16 || params.longEdge > 4096) throw new Error('Export longEdge must be 16..4096');
    const metadata = params.metadata === undefined ? exportDefaults.metadata : params.metadata;
    if (!metadataPolicies.includes(metadata)) throw new Error('Export metadata must be none, copyright or allExceptCamera');
    const removeLocation = params.removeLocation === undefined ? exportDefaults.removeLocation : params.removeLocation;
    if (typeof removeLocation !== 'boolean') throw new Error('Export removeLocation must be a boolean');
    const mimeType = params.format === 'png' ? 'image/png' : 'image/jpeg';
    const reply = await hook('lts.export', { id: params.id, format: params.format, longEdge: params.longEdge, metadata, removeLocation });
    // The answer must be the export asked for: the photo, the format and the policy the hook applied.
    const asked = reply.photoId === params.id && reply.format === params.format && reply.mime === mimeType
      && reply.metadata === metadata && reply.removeLocation === removeLocation;
    if (!asked) throw new Error('Image adjustments answered lts.export with another export than the one asked for');
    const bytes = exportedBytes(reply);
    const { bytesBase64: _bytes, ...exported } = reply;
    // The policy travels with the bytes, so the output record states what the engine was given; the
    // host names saved artifacts from the MIME type, so the name only travels in the result.
    return { name: `image-adjustments.${params.format === 'png' ? 'png' : 'jpg'}`, mimeType, bytes, metadata, removeLocation, export: exported, transfer: 'lts-export' };
  });
  api.ltsByteBridge = Object.freeze({ version: LTS_BYTE_BRIDGE_VERSION, ltsView: LTS_VIEW_VERSION, metadataPolicies, defaults: exportDefaults });
  return api;
}
