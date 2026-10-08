// LTS host extension for the pinned browser release. Uses the app's existing File-drop and
// download services; rendering and develop commands remain in the Rust engine. No network paths,
// model credentials, filesystem paths or arbitrary commands cross this byte interface.
export function installLtsByteBridge(api) {
  const maxInputBytes = 12 * 1024 * 1024;
  const maxOutputBytes = 32 * 1024 * 1024;
  let busy = false;
  async function exclusive(work) {
    if (busy) throw new Error('LightCraft byte transfer is already running');
    busy = true;
    try { return await work(); } finally { busy = false; }
  }
  async function command(id, params = {}) {
    return JSON.parse(await api.command(id, JSON.stringify(params)));
  }
  function checkedBytes(value) {
    if (!Array.isArray(value) || !value.length || value.length > maxInputBytes || value.some((n) => !Number.isInteger(n) || n < 0 || n > 255)) {
      throw new Error('Invalid or excessive image bytes');
    }
    return new Uint8Array(value);
  }
  api.importImage = (params) => exclusive(async () => {
    if (!params || typeof params.name !== 'string' || !/^[^\\/\x00-\x1f]{1,160}\.(png|jpe?g)$/i.test(params.name)) throw new Error('Invalid image name');
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
    const deadline = performance.now() + 15000;
    while (performance.now() < deadline) {
      const result = await command('catalog.query', { filter: {}, limit: 10000 });
      const photo = result.photos?.find((item) => item.fileName === fileName);
      if (photo) return { ...photo, sourceName: params.name, transfer: 'browser-file-drop' };
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    throw new Error('LightCraft did not finish importing the image within 15 seconds');
  });
  api.exportImage = (params) => exclusive(async () => {
    if (!params || !Number.isSafeInteger(params.id) || params.id < 1) throw new Error('Invalid photo id');
    if (!['png', 'jpeg'].includes(params.format)) throw new Error('Only PNG and JPEG exports are supported');
    if (!Number.isSafeInteger(params.longEdge) || params.longEdge < 16 || params.longEdge > 4096) throw new Error('Export longEdge must be 16..4096');
    const name = `lts-${crypto.randomUUID()}.${params.format === 'png' ? 'png' : 'jpg'}`;
    const originalClick = HTMLAnchorElement.prototype.click;
    const captured = [];
    HTMLAnchorElement.prototype.click = function () {
      if (this.download === name && this.href.startsWith('blob:')) {
        // Read the app-created Blob before its scheduled URL revocation. An exact random output
        // name keeps unrelated user downloads on their normal path.
        captured.push(fetch(this.href).then(async (response) => {
          const blob = await response.blob();
          if (!blob.size || blob.size > maxOutputBytes) throw new Error('Export exceeds the byte limit');
          return { name: `lightcraft.${params.format === 'png' ? 'png' : 'jpg'}`, mimeType: blob.type, bytes: Array.from(new Uint8Array(await blob.arrayBuffer())) };
        }));
        return;
      }
      return originalClick.call(this);
    };
    try {
      const result = await command('app.export', { ids: [params.id], path: name, format: params.format, longEdge: params.longEdge, dontEnlarge: true });
      if (captured.length !== 1) throw new Error('LightCraft did not produce exactly one requested download');
      return { ...(await captured[0]), export: result, transfer: 'browser-download-capture' };
    } finally {
      HTMLAnchorElement.prototype.click = originalClick;
    }
  });
  return api;
}
