import test from 'node:test';
import assert from 'node:assert/strict';
import { installLtsByteBridge } from './lts-byte-bridge.js';

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

test('owned export captures only its exact Blob download and restores normal clicks', async () => {
  let normalClicks = 0;
  class Anchor { click() { normalClicks += 1; } }
  const click = Anchor.prototype.click;
  const previous = { Anchor: globalThis.HTMLAnchorElement, fetch: globalThis.fetch };
  globalThis.HTMLAnchorElement = Anchor;
  globalThis.fetch = async () => ({ blob: async () => new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' }) });
  try {
    const api = installLtsByteBridge({ command: async (id, raw) => {
      assert.equal(id, 'app.export');
      const params = JSON.parse(raw);
      assert.deepEqual(params.ids, [7]);
      const unrelated = new Anchor(); unrelated.download = 'user.png'; unrelated.href = 'blob:other'; unrelated.click();
      const owned = new Anchor(); owned.download = params.path; owned.href = 'blob:owned'; owned.click();
      return JSON.stringify({ files: [{ width: 64, height: 48 }] });
    } });
    const output = await api.exportImage({ id: 7, format: 'png', longEdge: 64 });
    assert.deepEqual(output.bytes, [1, 2, 3]);
    assert.equal(normalClicks, 1);
    assert.equal(Anchor.prototype.click, click);
  } finally { globalThis.HTMLAnchorElement = previous.Anchor; globalThis.fetch = previous.fetch; }
});

test('missing engine download rejects and releases interception', async () => {
  class Anchor { click() {} }
  const click = Anchor.prototype.click;
  const previous = globalThis.HTMLAnchorElement;
  globalThis.HTMLAnchorElement = Anchor;
  try {
    const api = installLtsByteBridge({ command: async () => '{}' });
    await assert.rejects(api.exportImage({ id: 7, format: 'png', longEdge: 64 }), /exactly one/);
    assert.equal(Anchor.prototype.click, click);
  } finally { globalThis.HTMLAnchorElement = previous; }
});
