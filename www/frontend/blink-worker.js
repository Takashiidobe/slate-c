self.onmessage = async ({ data }) => {
  let completed = false;
  let module;
  const stdout = [];
  const stderr = [];
  const decoder = new TextDecoder();
  const collect = output => byte => {
    if (output.length >= 1048576) throw new Error('Program output exceeds 1 MiB.');
    output.push(byte);
  };
  try {
    const { default: create } = await import(data.js);
    module = await create({
      instantiateWasm(imports, receive) {
        WebAssembly.instantiate(data.wasm, imports).then(({ instance, module }) => receive(instance, module))
          .catch(error => postMessage({ error: String(error.message ?? error) }));
        return {};
      },
      stdout: collect(stdout), stderr: collect(stderr), stdin: () => null,
      onExit: code => {
        const executable = data.link && code === 0 ? module.FS.readFile('/work/main') : undefined;
        completed = true;
        postMessage({ code: code & 255, stdout: decoder.decode(new Uint8Array(stdout)),
          stderr: decoder.decode(new Uint8Array(stderr)), executable },
        executable ? [executable.buffer] : []);
      },
    });
    module.FS.mkdirTree('/work');
    for (const { path, bytes } of data.files) {
      module.FS.mkdirTree(path.slice(0, path.lastIndexOf('/')) || '/');
      module.FS.writeFile(path, bytes);
      module.FS.chmod(path, 0o755);
    }
    module.callMain(['-m', ...data.args]);
  } catch (error) {
    if (!completed) postMessage({ error: String(error.message ?? error),
      stdout: decoder.decode(new Uint8Array(stdout)), stderr: decoder.decode(new Uint8Array(stderr)) });
  }
};
