// Runs the actual inline presentation script of index.html in a minimal DOM fixture (the Film
// editor's pattern) and checks the page's host contract. It does not certify browser rendering,
// WASM start-up or the module script's import of the engine.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { createContext, runInContext } from 'node:vm';
import test from 'node:test';

const htmlPath = process.env.LIGHTCRAFT_INDEX_HTML ?? fileURLToPath(new URL('../index.html', import.meta.url));
const html = readFileSync(htmlPath, 'utf8');
const presentation = html.match(/<script id="lts-editor-presentation">([\s\S]*?)<\/script>/)?.[1];
const moduleScript = html.match(/<script type="module">([\s\S]*?)<\/script>/)?.[1];

function initial(pattern, what) {
  const match = html.match(pattern);
  assert.ok(match, `index.html has its initial ${what}`);
  return match[1];
}

function fixture(search) {
  assert.ok(presentation, 'index.html has the inline <script id="lts-editor-presentation">');
  const loading = { textContent: initial(/<div id="lightcraft_loading">([^<]*)</, 'loading text') };
  const description = {
    content: initial(/<meta name="description" content="([^"]*)">/, 'description'),
    setAttribute(name, value) {
      assert.equal(name, 'content');
      this.content = value;
    },
  };
  const document = {
    title: initial(/<title>([^<]*)<\/title>/, 'title'),
    getElementById(id) {
      assert.equal(id, 'lightcraft_loading', 'the script only renames the loading text');
      return loading;
    },
    querySelector(selector) {
      assert.equal(selector, 'meta[name="description"]');
      return description;
    },
  };
  const context = createContext({ document, location: { search }, URLSearchParams });
  runInContext(presentation, context, { filename: htmlPath, timeout: 1000 });
  // the module script reads this prefix when the engine fails to load
  const failurePrefix = runInContext('lightcraftLoadFailurePrefix', context);
  return { document, loading, description, failurePrefix };
}

for (const search of ['?embedded', '?embedded=', '?embedded=1', '?embedded=false', '?%65mbedded', '?store=memory&workers=1&embedded']) {
  test(`embedded presence flag names the LTS editor: ${search}`, () => {
    const f = fixture(search);
    assert.equal(f.document.title, 'LTS Image adjustments');
    assert.equal(f.loading.textContent, 'Loading LTS Image adjustments…');
    assert.equal(f.failurePrefix, 'LTS Image adjustments failed to load: ');
    assert.doesNotMatch(f.description.content, /LightCraft|ArtCraft/);
  });
}

for (const search of ['', '?embeddedness=1', '?mode=embedded', '?Embedded', '?store=memory&workers=1', '??embedded']) {
  test(`standalone page copy is unchanged: ${JSON.stringify(search)}`, () => {
    const f = fixture(search);
    assert.equal(f.document.title, 'LightCraft');
    assert.equal(f.loading.textContent, 'Loading LightCraft…');
    assert.equal(f.failurePrefix, 'LightCraft failed to load: ');
    assert.equal(f.description.content, 'LightCraft: open-source photo library and raw developer, running in your browser (WASM).');
  });
}

test('the host contract of index.html is unchanged', () => {
  // the LTS server injects its frame script with one `</body>` replacement
  assert.equal(html.split('</body>').length - 1, 1, 'exactly one literal </body>');
  for (const id of ['lightcraft_canvas', 'lightcraft_loading', 'lightcraft_status']) {
    assert.match(html, new RegExp(`id="${id}"`), id);
  }
  assert.ok(moduleScript, 'module script');
  assert.ok(html.indexOf('id="lts-editor-presentation"') < html.indexOf('<script type="module">'), 'presentation runs first');
  assert.match(moduleScript, /import init, \{ start, command \} from "\.\/lightcraft_web\.js";/);
  assert.match(moduleScript, /import \{ installLtsByteBridge \} from "\.\/lts-byte-bridge\.js";/);
  assert.match(moduleScript, /fetch\("\.\/lightcraft_web_bg\.wasm"\)/);
  assert.match(moduleScript, /globalThis\.lightcraftModule = module;/);
  assert.match(moduleScript, /globalThis\.lightcraft = installLtsByteBridge\(\{ command \}\);/);
  assert.match(moduleScript, /el\.textContent = lightcraftLoadFailurePrefix \+ e;/);
  // presentation only: no storage, network, navigation or engine access
  assert.doesNotMatch(presentation ?? '', /fetch|localStorage|indexedDB|location\.(href|assign|replace|reload)|window\.open|lightcraft\./);
});
