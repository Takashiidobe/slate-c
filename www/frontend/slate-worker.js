import init, { compileProgram } from './pkg/slate_c.js';

const ready = init();
ready.then(
  () => self.postMessage({ type: 'ready' }),
  error => self.postMessage({ type: 'load-error', error: String(error) }),
);

self.addEventListener('message', async ({ data }) => {
  const { id, source } = data;
  try {
    await ready;
    const start = performance.now();
    const compilation = compileProgram(source);
    let wasm;
    let clif;
    try {
      wasm = compilation.wasm;
      clif = compilation.clif;
    } finally {
      compilation.free();
    }
    self.postMessage({ id, ok: true, wasm, clif, ms: performance.now() - start }, [wasm.buffer]);
  } catch (error) {
    self.postMessage({ id, ok: false, error: String(error) });
  }
});
