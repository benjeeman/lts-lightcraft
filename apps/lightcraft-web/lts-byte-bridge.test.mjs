import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import * as bridge from './lts-byte-bridge.js';

const { installLtsByteBridge } = bridge;
// The LTS hook's result version: every answer of the hook carries it (lts_view.rs RESULT_VERSION).
const VIEW = 'lightcraft.lts-view.v1';

// An LTS page whose hook answers lts.export with `bytes` (bridge v3, R1: the page runs no other id);
// records each call's params.
function exportingPage(calls, bytes = [1, 2, 3]) {
  return {
    command: async (id, raw) => {
      const params = JSON.parse(raw);
      calls.push({ id, params });
      assert.equal(id, 'lts.export', 'an export reads the hook, nothing else');
      return JSON.stringify(exportAnswer(params, bytes));
    },
  };
}

// lts.export's answer to `params`: the hook's envelope and the export it describes.
function exportAnswer(params, bytes = [1, 2, 3]) {
  return {
    resultVersion: VIEW, command: 'lts.export', outcome: 'ok', photoId: params.id, format: params.format,
    mime: params.format === 'png' ? 'image/png' : 'image/jpeg', width: 64, height: 48, byteLength: bytes.length,
    metadata: params.metadata, removeLocation: params.removeLocation, hash: '0123456789abcdef', catalogRevision: 1,
    bytesBase64: Buffer.from(bytes).toString('base64'),
  };
}

// Browser stand-ins for the import path: image decoding for the size check, and the canvas that
// receives the File drop.
async function withFileDrop(run) {
  const names = ['createImageBitmap', 'document', 'DataTransfer', 'DragEvent'];
  const previous = new Map(names.map((name) => [name, Object.getOwnPropertyDescriptor(globalThis, name)]));
  const dropped = [];
  globalThis.createImageBitmap = async () => ({ width: 64, height: 48, close() {} });
  globalThis.DataTransfer = class { files = []; items = { add: (file) => { this.files.push(file); } }; };
  globalThis.DragEvent = class { constructor(type, init) { this.type = type; this.dataTransfer = init.dataTransfer; } };
  const canvas = { dispatchEvent(event) { assert.equal(event.type, 'drop'); dropped.push(...event.dataTransfer.files); return true; } };
  globalThis.document = { getElementById: (id) => (id === 'lightcraft_canvas' ? canvas : null) };
  try { return await run(dropped); } finally {
    for (const [name, descriptor] of previous) {
      if (descriptor) Object.defineProperty(globalThis, name, descriptor); else delete globalThis[name];
    }
  }
}

// An LTS page whose hook lists every dropped file as a photo of the library (lts.library {fileName}).
function libraryPage(dropped) {
  return {
    command: async (id, raw) => {
      assert.equal(id, 'lts.library');
      const params = JSON.parse(raw);
      assert.deepEqual(Object.keys(params), ['fileName']);
      const photos = dropped.map((file, index) => ({ id: index + 1, fileName: file.name })).filter((row) => row.fileName === params.fileName);
      return JSON.stringify({ resultVersion: VIEW, command: id, outcome: 'ok', total: photos.length, photos, catalogRevision: 1 });
    },
  };
}

test('byte imports reject traversal, malformed bytes and unsupported signatures before touching the app', async () => {
  const api = installLtsByteBridge({ command: async () => { throw new Error('must not call engine'); } });
  await assert.rejects(api.importImage({ name: '../secret.png', bytes: [137, 80] }), /name/);
  await assert.rejects(api.importImage({ name: 'a.png', bytes: [256] }), /bytes/);
  await assert.rejects(api.importImage({ name: 'a.png', bytes: [1, 2] }), /PNG and JPEG/);
});

test('export validates format and explicit bounded dimensions', async () => {
  const api = installLtsByteBridge({ command: async () => { throw new Error('must not call engine'); } });
  await assert.rejects(api.exportImage({ id: 1, format: 'original', longEdge: 64 }), /PNG and JPEG/);
  await assert.rejects(api.exportImage({ id: 1, format: 'png', longEdge: 0 }), /longEdge/);
  await assert.rejects(api.exportImage({ id: Number.MAX_SAFE_INTEGER + 1, format: 'png', longEdge: 64 }), /photo/);
});

// v3 (R1): the export is the hook's pure read, so the page's own downloads keep their normal path
// (v2 captured the engine's download and had to keep every other click on its way).
test('export reads its bytes from the hook and leaves the page\'s downloads alone', async () => {
  let normalClicks = 0;
  class Anchor { click() { normalClicks += 1; } }
  const click = Anchor.prototype.click;
  const previous = globalThis.HTMLAnchorElement;
  globalThis.HTMLAnchorElement = Anchor;
  try {
    const api = installLtsByteBridge({ command: async (id, raw) => {
      assert.equal(id, 'lts.export');
      const params = JSON.parse(raw);
      assert.equal(params.id, 7);
      const unrelated = new Anchor(); unrelated.download = 'user.png'; unrelated.href = 'blob:other'; unrelated.click();
      return JSON.stringify(exportAnswer(params));
    } });
    const output = await api.exportImage({ id: 7, format: 'png', longEdge: 64 });
    assert.deepEqual(Array.from(output.bytes), [1, 2, 3]);
    // the host names saved artifacts from the MIME type; this name only travels in the result
    assert.equal(output.name, 'image-adjustments.png');
    assert.equal(output.mimeType, 'image/png');
    assert.equal(normalClicks, 1);
    assert.equal(Anchor.prototype.click, click);
  } finally { globalThis.HTMLAnchorElement = previous; }
});

test('an answer that is not the hook\'s rejects and releases the transfer', async () => {
  const answers = ['{}', 'not json'];
  const api = installLtsByteBridge({ command: async (id, raw) => (answers.length ? answers.shift() : JSON.stringify(exportAnswer(JSON.parse(raw)))) });
  for (let attempt = 0; attempt < 2; attempt += 1) {
    await assert.rejects(api.exportImage({ id: 7, format: 'png', longEdge: 64 }), /^Error: Image adjustments answered lts\.export with an unexpected reply$/);
  }
  assert.deepEqual(Array.from((await api.exportImage({ id: 7, format: 'png', longEdge: 64 })).bytes), [1, 2, 3], 'the transfer lock is released');
});

test('overlapping byte transfers are refused in the editor-neutral wording', async () => {
  let release;
  const api = installLtsByteBridge({ command: () => new Promise((resolve) => { release = () => resolve('{}'); }) });
  const first = api.exportImage({ id: 7, format: 'png', longEdge: 64 });
  await assert.rejects(api.exportImage({ id: 8, format: 'jpeg', longEdge: 64 }), /^Error: Image adjustments byte transfer is already running$/);
  release();
  await assert.rejects(first, /^Error: Image adjustments answered lts\.export with an unexpected reply$/);
});

test('the bridge names no upstream product in what it says or produces', () => {
  const source = readFileSync(new URL('./lts-byte-bridge.js', import.meta.url), 'utf8');
  assert.doesNotMatch(source, /LightCraft|lightcraft\.(png|jpg)/);
  assert.match(source, /image-adjustments\.\$\{params\.format === 'png' \? 'png' : 'jpg'\}/);
});

// The plan's FL0 title says "none"; owner decision OC-13 (2026-10-14) sets the default to `copyright`
// (creator and copyright only, no camera data) with location removed. The bridge always sends both.
test('export passes metadata "copyright" and removeLocation true by default', async () => {
  const calls = [];
  const api = installLtsByteBridge(exportingPage(calls));
  const output = await api.exportImage({ id: 7, format: 'jpeg', longEdge: 64 });
  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0], { id: 'lts.export', params: { id: 7, format: 'jpeg', longEdge: 64, metadata: 'copyright', removeLocation: true } });
  // the result states the policy the engine was given, for the output record
  assert.equal(output.metadata, 'copyright');
  assert.equal(output.removeLocation, true);
});

// OC-13 keeps `none` selectable, so the accepted set is none, copyright and allExceptCamera.
test('export accepts none, copyright and allExceptCamera only', async () => {
  const calls = [];
  const api = installLtsByteBridge(exportingPage(calls));
  for (const metadata of ['none', 'copyright', 'allExceptCamera']) {
    for (const removeLocation of [true, false]) {
      const output = await api.exportImage({ id: 7, format: 'png', longEdge: 64, metadata, removeLocation });
      const sent = calls.at(-1).params;
      assert.deepEqual([sent.metadata, sent.removeLocation], [metadata, removeLocation]);
      assert.deepEqual([output.metadata, output.removeLocation], [metadata, removeLocation]);
    }
  }
  // an explicit policy keeps the location default
  await api.exportImage({ id: 7, format: 'png', longEdge: 64, metadata: 'allExceptCamera' });
  assert.deepEqual([calls.at(-1).params.metadata, calls.at(-1).params.removeLocation], ['allExceptCamera', true]);
  const sentCount = calls.length;
  // `all` adds camera, lens and capture settings; the hook refuses anything else as well
  for (const metadata of ['all', 'All', 'NONE', 'Copyright', 'allexceptcamera', 'none ', '', null, false, 0, ['none'], { metadata: 'none' }]) {
    await assert.rejects(api.exportImage({ id: 7, format: 'png', longEdge: 64, metadata }),
      /^Error: Export metadata must be none, copyright or allExceptCamera$/, JSON.stringify(metadata));
  }
  // a location kept by mistake is a privacy leak: removeLocation is a boolean or absent
  for (const removeLocation of ['true', 'false', 1, 0, null, [true]]) {
    await assert.rejects(api.exportImage({ id: 7, format: 'png', longEdge: 64, removeLocation }),
      /^Error: Export removeLocation must be a boolean$/, JSON.stringify(removeLocation));
  }
  assert.equal(calls.length, sentCount, 'refused exports never reach the engine');
  await api.exportImage({ id: 7, format: 'png', longEdge: 64 });
  assert.equal(calls.length, sentCount + 1, 'refusals release the transfer lock');
});

test('import accepts Uint8Array and arrays', async () => {
  const png = [137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13];
  await withFileDrop(async (dropped) => {
    const api = installLtsByteBridge(libraryPage(dropped));
    const typed = new Uint8Array(png);
    const pending = api.importImage({ name: 'typed.png', bytes: typed });
    typed.fill(0); // the caller may reuse its buffer while the import runs
    const fromTyped = await pending;
    const fromArray = await api.importImage({ name: 'plain.png', bytes: png });
    // a same-origin caller in another frame passes its own realm's Uint8Array, where instanceof fails
    const foreign = runInNewContext('Uint8Array.from(bytes)', { bytes: png });
    assert.equal(foreign instanceof Uint8Array, false);
    const fromForeign = await api.importImage({ name: 'foreign.png', bytes: foreign });
    const imports = [[fromTyped, 'typed'], [fromArray, 'plain'], [fromForeign, 'foreign']];
    assert.equal(dropped.length, imports.length);
    for (const [index, [imported, stem]] of imports.entries()) {
      const file = dropped[index];
      assert.match(file.name, new RegExp(`^lts-[0-9a-f-]{36}-${stem}\\.png$`));
      assert.equal(file.type, 'image/png');
      assert.deepEqual([...new Uint8Array(await file.arrayBuffer())], png);
      assert.deepEqual(imported, { id: index + 1, fileName: file.name, sourceName: `${stem}.png`, transfer: 'browser-file-drop' });
    }
  });
});

test('import refuses empty, oversized, holey and non-Uint8Array byte containers before touching the app', async () => {
  const api = installLtsByteBridge({ command: async () => { throw new Error('must not call engine'); } });
  const png = [137, 80, 78, 71, 13, 10, 26, 10];
  const refused = [
    new Uint8Array(0), new Uint8Array(12 * 1024 * 1024 + 1), [],
    // a hole would otherwise become a zero byte
    [...png, , 0], [...png, 1.5], [...png, -1], [...png, 256], [...png, '1'],
    new Uint8ClampedArray(png), new Int8Array(png), new Uint16Array(png), new Uint8Array(png).buffer,
    new DataView(new Uint8Array(png).buffer), { length: png.length, ...png }, String.fromCharCode(...png),
    Object.setPrototypeOf({ length: png.length, ...png }, Uint8Array.prototype),
  ];
  for (const [index, bytes] of refused.entries()) {
    await assert.rejects(api.importImage({ name: 'a.png', bytes }), /^Error: Invalid or excessive image bytes$/, `case ${index}`);
  }
});

test('byte transfers refuse unknown parameters before touching the app', async () => {
  // a misspelt privacy key must not fall back to the default policy silently
  const api = installLtsByteBridge({ command: async () => { throw new Error('must not call engine'); } });
  await assert.rejects(api.exportImage({ id: 7, format: 'png', longEdge: 64, metadataPolicy: 'none' }), /^Error: Unknown export parameter: metadataPolicy$/);
  await assert.rejects(api.exportImage({ id: 7, format: 'png', longEdge: 64, removeGps: true }), /^Error: Unknown export parameter: removeGps$/);
  await assert.rejects(api.exportImage([7, 'png', 64]), /^Error: Export parameters must be an object$/);
  await assert.rejects(api.importImage({ name: 'a.png', bytes: [137, 80, 78, 71], path: 'a.png' }), /^Error: Unknown import parameter: path$/);
  await assert.rejects(api.importImage(null), /^Error: Import parameters must be an object$/);
});

test('export returns Uint8Array bytes', async () => {
  const api = installLtsByteBridge(exportingPage([]));
  const output = await api.exportImage({ id: 7, format: 'png', longEdge: 64 });
  assert.ok(output.bytes instanceof Uint8Array, 'a Uint8Array, not a plain array');
  assert.deepEqual([...output.bytes], [1, 2, 3]);
  // it owns its whole buffer, so the host can transfer it without a copy
  assert.equal(output.bytes.byteOffset, 0);
  assert.equal(output.bytes.buffer.byteLength, 3);
});

test('bridge reports version lts-byte-bridge.v3', () => {
  const command = async () => '{}';
  const host = { command };
  const api = installLtsByteBridge(host);
  assert.equal(api, host, 'installs on the object it is given');
  assert.equal(api.command, command, "keeps the engine's own command");
  assert.equal(bridge.LTS_BYTE_BRIDGE_VERSION, 'lts-byte-bridge.v3');
  assert.deepEqual(api.ltsByteBridge, {
    version: 'lts-byte-bridge.v3',
    ltsView: VIEW,
    metadataPolicies: ['none', 'copyright', 'allExceptCamera'],
    defaults: { metadata: 'copyright', removeLocation: true },
  });
  for (const value of [api.ltsByteBridge, api.ltsByteBridge.metadataPolicies, api.ltsByteBridge.defaults]) assert.ok(Object.isFrozen(value));
});
