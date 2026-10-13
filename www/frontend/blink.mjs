function execute(data, signal, timeout) {
  return new Promise((resolve, reject) => {
    if (signal.aborted) { reject(new Error('Execution cancelled.')); return; }
    const worker = new Worker(new URL('./blink-worker.js', import.meta.url), { type: 'module' });
    const finish = (error, result) => {
      clearTimeout(timer);
      signal.removeEventListener('abort', cancel);
      worker.terminate();
      if (error) reject(error); else resolve(result);
    };
    const cancel = () => finish(new Error('Execution cancelled.'));
    const timer = setTimeout(() => finish(new Error(data.link ? 'Linking timed out.' : 'Execution timed out.')), timeout);
    signal.addEventListener('abort', cancel, { once: true });
    worker.onmessage = ({ data }) => finish(data.error ? new Error(data.stderr || data.error) : null, data);
    worker.onerror = event => finish(new Error(event.message || 'Blink worker failed.'));
    worker.postMessage(data);
  });
}

export async function runProgram(object, assets, signal, progress) {
  progress('Linking with musl…');
  const linked = await execute({ ...assets, link: true,
    files: [...assets.files, { path: '/work/main.o', bytes: object }],
    args: ['/usr/bin/ld.bfd', '-static', '-o', '/work/main', '/usr/lib/crt1.o',
      '/usr/lib/crti.o', '/work/main.o', '-L/usr/lib', '--start-group', '-lslate',
      '-lc', '-lgcc', '--end-group', '/usr/lib/crtn.o'],
  }, signal, 30000);
  if (linked.code !== 0) throw new Error(linked.stderr || `Linker exited with status ${linked.code}.`);
  if (!linked.executable) throw new Error('Linker did not produce an executable.');
  progress('Running…');
  const result = await execute({ js: assets.js, wasm: assets.wasm,
    files: [{ path: '/work/main', bytes: linked.executable }], args: ['/work/main'],
  }, signal, 5000);
  return { status: 'ok', exitCode: result.code, stdout: result.stdout, stderr: result.stderr,
    executable: linked.executable };
}
