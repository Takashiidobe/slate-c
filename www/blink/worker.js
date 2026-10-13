import create from './blink.mjs';

self.onmessage = async ({ data: { executable } }) => {
  let finished = false;
  const complete = code => {
    if (finished) return;
    finished = true;
    postMessage({ type: 'exit', code });
  };
  try {
    const bytes = new Uint8Array(executable);
    const header = new DataView(executable);
    if (bytes.length < 64 || header.getUint32(0, false) !== 0x7f454c46 ||
        bytes[4] !== 2 || bytes[5] !== 1 || header.getUint16(18, true) !== 62 ||
        ![2, 3].includes(header.getUint16(16, true))) {
      throw new Error('Upload an x86_64 Linux ELF executable, not a relocatable object.');
    }
    const module = await create({
      print: text => postMessage({ type: 'output', text }),
      printErr: text => postMessage({ type: 'output', text }),
      onExit: complete,
      stdin: () => null,
    });
    module.FS.writeFile('/program', bytes);
    module.FS.chmod('/program', 0o755);
    module.callMain(['-m', '/program']);
  } catch (error) {
    if (!finished) postMessage({ type: 'error', error: String(error.message ?? error) });
  }
};
