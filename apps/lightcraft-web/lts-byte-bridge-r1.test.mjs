// R1 (master plan v3 W3, fork-light): byte bridge v3 rides the LTS hook. A WASM built from this fork
// refuses every id outside the hook's table when the page has `?embedded` (lts_view.rs `handle`,
// T-LI-21): `catalog.query` and `app.export`, which bridge v2 sent, no longer run there. So the bridge
// finds an imported photo with `lts.library {fileName}` and reads exported bytes with `lts.export` (a
// pure output read: no download, no last-export preference), sends no other id, and declares the hook
// it rides next to its own version (`ltsView`), which the adapter reads instead of probing.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import * as bridge from './lts-byte-bridge.js';

const { installLtsByteBridge } = bridge;
const VIEW = 'lightcraft.lts-view.v1';
const PNG = [137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13];

// The LTS page: the hook answers its own ids (`answers[id](params)`) and every other id is refused at
// the fork, typed, as `handle` refuses it; every call is recorded.
function ltsPage(answers, calls) {
  return {
    command: async (id, raw) => {
      const params = JSON.parse(raw);
      calls.push({ id, params });
      if (!Object.hasOwn(answers, id)) {
        return JSON.stringify({ resultVersion: VIEW, command: id, outcome: 'not_applied', code: 'command_not_allowed',
          message: 'not a command of this hook: lts.state, lts.preview, lts.export, ...' });
      }
      return JSON.stringify({ resultVersion: VIEW, command: id, ...answers[id](params) });
    },
  };
}

// `lts.export`'s answer for `params`, the bytes [1, 2, 3].
function exported(params) {
  return {
    outcome: 'ok', photoId: params.id, format: params.format, mime: params.format === 'png' ? 'image/png' : 'image/jpeg',
    width: 64, height: 48, byteLength: 3, metadata: params.metadata, removeLocation: params.removeLocation,
    hash: '0123456789abcdef', catalogRevision: 9, bytesBase64: 'AQID',
  };
}

// Browser stand-ins for the import path: image decoding for the size check, and the canvas that
// receives the File drop (as in lts-byte-bridge.test.mjs).
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

test('bridge reports version lts-byte-bridge.v3 and the hook it rides (ltsView)', () => {
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

test('export reads the bytes with lts.export, a pure output read, under the default policy', async () => {
  const calls = [];
  const api = installLtsByteBridge(ltsPage({ 'lts.export': exported }, calls));
  // no download is captured: the page needs no anchor at all
  assert.equal(globalThis.HTMLAnchorElement, undefined);
  const output = await api.exportImage({ id: 7, format: 'png', longEdge: 64 });
  assert.deepEqual(calls, [{ id: 'lts.export', params: { id: 7, format: 'png', longEdge: 64, metadata: 'copyright', removeLocation: true } }]);
  assert.ok(output.bytes instanceof Uint8Array, 'a Uint8Array');
  assert.deepEqual([...output.bytes], [1, 2, 3]);
  // it owns its whole buffer, so the host can transfer it without a copy
  assert.equal(output.bytes.byteOffset, 0);
  assert.equal(output.bytes.buffer.byteLength, 3);
  const { bytes: _bytes, ...rest } = output;
  const { bytesBase64: _base64, ...reply } = { resultVersion: VIEW, command: 'lts.export', ...exported(calls[0].params) };
  assert.deepEqual(rest, { name: 'image-adjustments.png', mimeType: 'image/png', metadata: 'copyright', removeLocation: true, export: reply, transfer: 'lts-export' });
});

test('export passes every accepted policy to lts.export and names a JPEG by its MIME type', async () => {
  const calls = [];
  const api = installLtsByteBridge(ltsPage({ 'lts.export': exported }, calls));
  for (const metadata of ['none', 'copyright', 'allExceptCamera']) {
    for (const removeLocation of [true, false]) {
      const output = await api.exportImage({ id: 3, format: 'jpeg', longEdge: 128, metadata, removeLocation });
      assert.deepEqual(calls.at(-1), { id: 'lts.export', params: { id: 3, format: 'jpeg', longEdge: 128, metadata, removeLocation } });
      assert.deepEqual([output.name, output.mimeType, output.metadata, output.removeLocation], ['image-adjustments.jpg', 'image/jpeg', metadata, removeLocation]);
    }
  }
});

test("export reports the hook's refusal by its code and releases the transfer", async () => {
  const calls = [];
  const refused = () => ({ outcome: 'not_applied', code: 'target_not_found', message: 'there is no photo 7 in the library' });
  const api = installLtsByteBridge(ltsPage({ 'lts.export': refused }, calls));
  await assert.rejects(api.exportImage({ id: 7, format: 'png', longEdge: 64 }), /target_not_found/);
  await assert.rejects(api.exportImage({ id: 7, format: 'png', longEdge: 64 }), /target_not_found/, 'the transfer lock is released');
  assert.deepEqual(calls.map((call) => call.id), ['lts.export', 'lts.export']);
});

test('export refuses an answer whose bytes are not the ones it describes', async () => {
  const sound = installLtsByteBridge(ltsPage({ 'lts.export': exported }, []));
  assert.deepEqual([...(await sound.exportImage({ id: 7, format: 'png', longEdge: 64 })).bytes], [1, 2, 3], 'a sound answer is taken');
  for (const [what, answer] of [
    ['a byte length that is not the bytes', { byteLength: 4 }],
    ['bytes that are not base64', { bytesBase64: 'AQI*' }],
    ['no bytes', { bytesBase64: '', byteLength: 0 }],
    ['an answer without bytes', { bytesBase64: undefined }],
    ['an answer of another command', { command: 'lts.preview' }],
  ]) {
    const api = installLtsByteBridge(ltsPage({ 'lts.export': (params) => ({ ...exported(params), ...answer }) }, []));
    await assert.rejects(api.exportImage({ id: 7, format: 'png', longEdge: 64 }), /lts\.export/, what);
  }
});

test('import finds the dropped photo with lts.library by its exact file name, and sends nothing else', async () => {
  await withFileDrop(async (dropped) => {
    const calls = [];
    const library = (params) => {
      const photos = dropped.filter((file) => file.name === params.fileName)
        .map((file) => ({ id: 41, fileName: file.name, width: 64, height: 48, hash: 'fedcba9876543210' }));
      return { outcome: 'ok', total: photos.length, photos, catalogRevision: 3 };
    };
    const api = installLtsByteBridge(ltsPage({ 'lts.library': library }, calls));
    const imported = await api.importImage({ name: 'still.png', bytes: PNG });
    assert.equal(dropped.length, 1);
    const file = dropped[0];
    assert.match(file.name, /^lts-[0-9a-f-]{36}-still\.png$/);
    assert.deepEqual(imported, { id: 41, fileName: file.name, width: 64, height: 48, hash: 'fedcba9876543210', sourceName: 'still.png', transfer: 'browser-file-drop' });
    assert.ok(calls.length >= 1, 'the bridge asked the hook');
    for (const call of calls) assert.deepEqual(call, { id: 'lts.library', params: { fileName: file.name } });
  });
});

test("import reports the hook's refusal by its code at once", async () => {
  await withFileDrop(async () => {
    const calls = [];
    const refused = () => ({ outcome: 'not_applied', code: 'invalid_params', message: '`fileName` is 1 to 255 characters' });
    const api = installLtsByteBridge(ltsPage({ 'lts.library': refused }, calls));
    const started = Date.now();
    await assert.rejects(api.importImage({ name: 'still.png', bytes: PNG }), /invalid_params/);
    assert.ok(Date.now() - started < 5000, 'a refusal ends the wait');
    assert.deepEqual(calls.map((call) => call.id), ['lts.library']);
  });
});

test('the bridge names no engine command outside the hook and captures no download', () => {
  const source = readFileSync(new URL('./lts-byte-bridge.js', import.meta.url), 'utf8');
  const literal = [...source.matchAll(/command\(\s*['"`]([^'"`]*)['"`]/g)].map((match) => match[1]);
  for (const id of literal) assert.match(id, /^lts\./, `the bridge sends ${id}`);
  assert.doesNotMatch(source, /HTMLAnchorElement\.prototype|createObjectURL|\.download\s*===/, 'no download is captured');
});
