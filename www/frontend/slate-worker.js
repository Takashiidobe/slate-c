import { ConsoleStdout, Directory, File, OpenFile, PreopenDirectory, WASI } from './vendor/browser_wasi_shim/index.js';
import { createDisassembler } from './disasm.mjs';
import { runProgram } from './blink.mjs';

const encoder = new TextEncoder();
const decoder = new TextDecoder();
const cache = new Map();
let compiled;
let manifest;
let compiler;
let capstone;
let running;

function status(id, text) { postMessage({ type: 'status', id, text }); }

function bytes(path) {
  if (!cache.has(path)) cache.set(path, fetch(path).then(async response => {
    if (!response.ok) throw new Error(`${path}: ${response.status}`);
    return new Uint8Array(await response.arrayBuffer());
  }).catch(error => { cache.delete(path); throw error; }));
  return cache.get(path);
}

function untar(bytes) {
  const root = new Map();
  const field = (header, start, size) => decoder.decode(header.subarray(start, start + size)).replace(/\0.*$/s, '');
  for (let offset = 0; offset + 512 <= bytes.length;) {
    const header = bytes.subarray(offset, offset + 512);
    if (!header[0]) break;
    const size = parseInt(field(header, 124, 12).trim() || '0', 8);
    const prefix = field(header, 345, 155);
    const name = `${prefix ? `${prefix}/` : ''}${field(header, 0, 100)}`;
    offset += 512;
    if (header[156] === 48 || header[156] === 0) {
      const parts = name.split('/').filter(part => part && part !== '.');
      const filename = parts.pop();
      let directory = root;
      for (const part of parts) {
        if (part === '..') throw new Error('Invalid header archive path.');
        if (!directory.has(part)) directory.set(part, new Map());
        directory = directory.get(part);
      }
      directory.set(filename, bytes.slice(offset, offset + size));
    }
    offset += Math.ceil(size / 512) * 512;
  }
  const directory = tree => new Directory(new Map([...tree].map(([name, value]) => [name,
    value instanceof Map ? directory(value) : new File(value, { readonly: true })])));
  return directory(root);
}

async function archive(path) {
  const key = `archive:${path}`;
  if (!cache.has(key)) cache.set(key, bytes(path).then(async data => {
    const unpacked = await new Response(new Blob([data]).stream().pipeThrough(new DecompressionStream('gzip'))).arrayBuffer();
    return untar(new Uint8Array(unpacked));
  }).catch(error => { cache.delete(key); throw error; }));
  return cache.get(key);
}

async function invoke(request, targetHeaders, builtinHeaders) {
  let out = '';
  let err = '';
  const work = new Map([['input.c', new File(encoder.encode(request.source ?? ''))]]);
  const fds = [new OpenFile(new File(encoder.encode(JSON.stringify(request)))),
    new ConsoleStdout(data => { out += decoder.decode(data, { stream: true }); }),
    new ConsoleStdout(data => { err += decoder.decode(data, { stream: true }); }),
    new PreopenDirectory('/work', work),
    new PreopenDirectory('/sysroots', new Map(targetHeaders ? [[request.target, targetHeaders]] : [])),
    new PreopenDirectory('/compiler-headers', builtinHeaders?.contents ?? new Map())];
  const wasi = new WASI(['slate-c-browser'], ['SLATE_SYSROOTS=/sysroots', 'SLATE_COMPILER_HEADERS=/compiler-headers'], fds);
  const instance = await WebAssembly.instantiate(compiler, { wasi_snapshot_preview1: wasi.wasiImport });
  const exit = wasi.start(instance);
  if (exit !== 0) throw new Error(err || `Compiler exited with status ${exit}.`);
  let response;
  try { response = JSON.parse(out); }
  catch { throw new Error(err || 'Compiler returned an invalid response.'); }
  if (!response.ok) throw new Error(response.error);
  return response;
}

function key(request) { return JSON.stringify([request.source, request.target, request.c_flags, request.cranelift_flags]); }

async function compile(request, id) {
  const { source, target: triple, c_flags, cranelift_flags } = request;
  request = { source, target: triple, c_flags, cranelift_flags };
  const target = manifest.targets[request.target];
  if (!target) throw new Error('Unsupported target.');
  status(id, 'Loading target and compiler headers…');
  const [headers, builtins] = await Promise.all([archive(target.headers), archive(manifest.compilerHeaders)]);
  status(id, 'Compiling C…');
  const started = performance.now();
  const { compilation } = await invoke(request, headers, builtins);
  const ms = performance.now() - started;
  compilation.object = new Uint8Array(compilation.object);
  for (const fn of compilation.functions) fn.code = new Uint8Array(fn.code);
  status(id, 'Loading disassembler…');
  capstone ??= bytes(manifest.capstone.wasm).then(async wasm => {
    const { default: create } = await import(new URL(manifest.capstone.js, self.location).href);
    return create({ wasmBinary: wasm });
  });
  const module = await capstone;
  for (const syntax of target.arch === 'x86_64' ? ['intel', 'att'] : ['intel']) {
    const disassembler = createDisassembler(target.arch, module, syntax);
    try {
      compilation[syntax === 'att' ? 'attAssembly' : 'assembly'] = compilation.functions.map(fn => `${fn.name}:\n${disassembler.disassemble(fn.code).map(line =>
        `  ${line.text}`).join('\n')}`).join('\n\n');
    } finally { disassembler.close(); }
  }
  const hexDump = bytes => {
    const lines = [];
    for (let offset = 0; offset < bytes.length; offset += 8) {
      lines.push(`${offset.toString(16).padStart(8, '0')}  ${Array.from(bytes.slice(offset, offset + 8), byte => byte.toString(16).padStart(2, '0')).join(' ')}`);
    }
    return lines.join('\n');
  };
  compilation.binary = compilation.functions.map(fn => `${fn.name} (${fn.code.length} bytes):\n${hexDump(fn.code)}`).join('\n\n');
  compilation.elf = hexDump(compilation.object);
  compiled = { key: key(request), compilation };
  return { ...compilation, ms };
}

async function run(request, id) {
  if (running) throw new Error('A program is already running.');
  if (compiled?.key !== key(request)) await compile(request, id);
  running = new AbortController();
  try {
    const started = performance.now();
    status(id, 'Loading Blink and linker…');
    const signal = running.signal;
    const load = Promise.all([bytes(manifest.blink.wasm), archive(manifest.blink.runtime)]);
    const [wasm, runtime] = await new Promise((resolve, reject) => {
      const cancel = () => reject(new Error('Execution cancelled.'));
      signal.addEventListener('abort', cancel, { once: true });
      load.then(resolve, reject).finally(() => signal.removeEventListener('abort', cancel));
      if (signal.aborted) cancel();
    });
    if (running.signal.aborted) throw new Error('Execution cancelled.');
    const files = [];
    const flatten = (directory, prefix = '') => {
      for (const [name, entry] of directory.contents) {
        const path = `${prefix}/${name}`;
        if (entry instanceof Directory) flatten(entry, path);
        else files.push({ path, bytes: entry.data });
      }
    };
    flatten(runtime);
    const result = await runProgram(compiled.compilation.object,
      { wasm, js: new URL(manifest.blink.js, self.location).href, files },
      running.signal, text => status(id, text));
    return { ...result, ms: performance.now() - started };
  } finally { running = null; }
}

const ready = fetch('manifest.json').then(async response => {
  if (!response.ok) throw new Error(`manifest.json: ${response.status}`);
  manifest = await response.json();
  compiler = await WebAssembly.compile(await bytes(manifest.compiler));
  postMessage({ type: 'ready', targets: Object.keys(manifest.targets) });
}).catch(error => postMessage({ type: 'load-error', error: String(error.message ?? error) }));

self.onmessage = async ({ data: { id, type, request } }) => {
  if (type === 'cancel') { running?.abort(); return; }
  try {
    await ready;
    const result = type === 'schema' ? (await invoke({ operation: 'schema', target: request.target })).schema
      : type === 'compile' ? await compile(request, id) : await run(request, id);
    postMessage({ id, ok: true, result });
  } catch (error) { postMessage({ id, ok: false, error: String(error.message ?? error) }); }
};
