# LTS browser byte bridge

The LTS fork retains the existing Rust engine, develop commands, browser canvas and native MCP server. Its `lts-byte-bridge.js` adds two browser host functions to the existing `globalThis.lightcraft` object:

- `importImage({name, bytes})`: a PNG/JPEG byte array becomes a browser File and follows the same File-drop import service as a human drop. The result is the catalogued photo ID and dimensions, after asynchronous import. Files receive a unique prefix so repeated names cannot select an older image. Encoded inputs are bounded to 12 MiB. The 16-megapixel admission check occurs **after** browser decoding, so it is not a hard decoder-memory bound; a future header dimension precheck could reject oversized images before that allocation.
- `exportImage({id, format, longEdge})`: invokes existing `app.export`, capturing its exact randomly named Blob download. It returns encoded bytes and the engine export metadata. PNG and JPEG are supported, long edge 16–4096, output at most 32 MiB. Other browser downloads retain their normal behavior.

This bridge is a host adapter around existing public browser services. It does not create another MCP implementation, grant access to host filesystem paths, provide durable LTS publication, or synchronize a native catalog with the browser. The LTS host owns source/origin/session validation and serializes operations. The bridge itself rejects overlapping byte transfers. Rust WASM work still runs on the browser thread for export; an in-flight synchronous export cannot be cancelled by a JavaScript timer.

An isolated memory-store session (`?store=memory`) does not persist the browser catalog; its edits must be captured or exported before closing. The existing default demo library remains visible. Persistent web libraries continue to use upstream Web Locks and storage behavior. No native catalog is opened by the browser.

For a release-compatible integration, `index.html` and `lts-byte-bridge.js` can be overlaid on the official hash-verified v0.4.0 web release without rebuilding its WASM. `xtask/src/web.rs` includes the bridge in future static bundles and compression. Label that runtime as a patched host with upstream WASM, not a newly compiled fork engine.

Validation: `node --test apps/lightcraft-web/lts-byte-bridge.test.mjs`. The LTS adapter additionally tests typed parameter validation and unsupported-function errors. On 2026-10-08, an actual Chromium browser test imported a procedural 320×240 PNG, set exposure +1/contrast +12, cropped the center half and returned a 160×120 PNG. Independently decoded pixels matched the equivalent native MCP output exactly. This limited PNG test is not RAW fidelity or general performance qualification.
