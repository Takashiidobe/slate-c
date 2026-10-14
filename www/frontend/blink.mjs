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

export async function linkProgram(object, assets, signal, progress, arch = 'x86_64') {
  progress('Linking with musl…');
  const started = performance.now();
  const aarch64 = arch === 'aarch64';
  const root = '/sysroots/aarch64/usr/lib';
  const linked = await execute({ ...assets, link: true,
    files: [...assets.files, { path: '/work/main.o', bytes: object }],
    args: aarch64
      ? ['/usr/aarch64-alpine-linux-musl/bin/ld.bfd', '--sysroot=/sysroots/aarch64', '-static',
        '-o', '/work/main', `${root}/crt1.o`, `${root}/crti.o`, '/work/main.o', `-L${root}`,
        '--start-group', '-lslate', '-lc', '-lgcc', '--end-group', `${root}/crtn.o`]
      : ['/usr/bin/ld.bfd', '-static', '-o', '/work/main', '/usr/lib/crt1.o',
        '/usr/lib/crti.o', '/work/main.o', '-L/usr/lib', '--start-group', '-lslate',
        '-lc', '-lgcc', '--end-group', '/usr/lib/crtn.o'],
  }, signal, 30000);
  if (linked.code !== 0) throw new Error(linked.stderr || `Linker exited with status ${linked.code}.`);
  if (!linked.executable) throw new Error('Linker did not produce an executable.');
  return { executable: linked.executable, linkMs: performance.now() - started };
}

export async function runProgram(object, assets, signal, progress) {
  const linked = await linkProgram(object, assets, signal, progress);
  progress('Running…');
  const started = performance.now();
  const result = await execute({ js: assets.js, wasm: assets.wasm,
    files: [{ path: '/work/main', bytes: linked.executable }], args: ['/work/main'],
  }, signal, 5000);
  return { status: 'ok', exitCode: result.code, stdout: result.stdout, stderr: result.stderr,
    executable: linked.executable, linkMs: linked.linkMs, guestMs: performance.now() - started };
}
