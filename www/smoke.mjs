import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { initSync, compileWasm, compileProgram } from './dist/pkg/slate_c.js';

const compiler = await readFile(new URL('./dist/pkg/slate_c_bg.wasm', import.meta.url));
initSync({ module: compiler });
const fixture = await readFile(new URL('../tests/fixtures/return42.c', import.meta.url), 'utf8');
for (const [source, expected] of [[fixture, 42], ['int main(void) { return 17; }', 17]]) {
  const compilation = compileProgram(source);
  const bytes = compilation.wasm;
  assert(compilation.clif.includes(`iconst.i32 ${expected}`));
  compilation.free();
  assert(WebAssembly.validate(bytes));
  const { instance } = await WebAssembly.instantiate(bytes);
  assert.equal(instance.exports.main(), expected);
}
assert.throws(() => compileWasm('int main(void) { int x = 42; return x; }'));
console.log('Wasm-hosted compiler emits programs returning 42 and 17; unsupported input rejected.');
