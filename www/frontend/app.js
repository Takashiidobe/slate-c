const source = CodeMirror.fromTextArea(document.querySelector('#c'), {
  mode: 'text/x-csrc', theme: 'material-darker', lineNumbers: true, indentUnit: 4, tabSize: 4,
});
const output = CodeMirror.fromTextArea(document.querySelector('#clif'), {
  mode: 'text/plain', theme: 'material-darker', lineNumbers: true, readOnly: true,
});
const button = document.querySelector('#compile');
const download = document.querySelector('#download');
const copyClif = document.querySelector('#copy-clif');
const status = document.querySelector('#status');
const diagnostics = document.querySelector('#diagnostics');
const execution = document.querySelector('#execution');
const size = document.querySelector('#wasm-size');
const shell = document.querySelector('#output-shell');
const worker = new Worker('slate-worker.js', { type: 'module' });
let ready = false;
let request = 0;
let revision = 0;
let compiledBytes;
let debounce;
let timeout;
let activeRevision;

function setStatus(text, kind = '') {
  status.textContent = text;
  status.className = kind;
}

function invalidate() {
  revision += 1;
  compiledBytes = undefined;
  download.disabled = true;
  copyClif.disabled = true;
  shell.classList.add('stale');
  diagnostics.hidden = true;
  execution.textContent = 'Waiting for compilation';
  execution.className = '';
  size.textContent = '';
}

function compile() {
  clearTimeout(debounce);
  if (!ready) return;
  invalidate();
  request += 1;
  activeRevision = revision;
  button.disabled = true;
  setStatus('Compiling…');
  clearTimeout(timeout);
  timeout = setTimeout(() => {
    worker.terminate();
    ready = false;
    fail('Compilation timed out. Reload the page to restart the compiler.');
  }, 30000);
  worker.postMessage({ id: request, source: source.getValue() });
}

function fail(message) {
  compiledBytes = undefined;
  download.disabled = true;
  copyClif.disabled = true;
  output.setValue('');
  diagnostics.textContent = message;
  diagnostics.hidden = false;
  shell.classList.remove('stale');
  execution.textContent = 'No program executed';
  execution.className = '';
  size.textContent = '';
  setStatus(message.startsWith('Compiler') ? message : 'Compilation failed.', 'error');
  button.disabled = !ready;
}

worker.addEventListener('message', async ({ data }) => {
  if (data.type === 'ready') {
    ready = true;
    button.disabled = false;
    compile();
    return;
  }
  if (data.type === 'load-error') {
    fail(`Compiler failed to load: ${data.error}`);
    return;
  }
  if (data.id !== request) return;
  clearTimeout(timeout);
  button.disabled = false;
  if (activeRevision !== revision) return;
  if (!data.ok) {
    fail(data.error);
    return;
  }
  const snapshot = revision;
  try {
    const { instance } = await WebAssembly.instantiate(data.wasm);
    if (snapshot !== revision) return;
    const result = instance.exports.main();
    compiledBytes = data.wasm;
    output.setValue(data.clif);
    shell.classList.remove('stale');
    diagnostics.hidden = true;
    execution.textContent = `main returned ${result}`;
    execution.className = 'ok';
    size.textContent = `${data.wasm.byteLength} bytes of Wasm`;
    download.disabled = false;
    copyClif.disabled = false;
    setStatus(`Compiled in ${Math.round(data.ms)} ms.`, 'ok');
  } catch (error) {
    if (snapshot === revision) fail(String(error));
  }
});

worker.addEventListener('error', event => {
  clearTimeout(timeout);
  ready = false;
  fail(`Compiler worker failed: ${event.message}`);
});

source.on('change', () => {
  invalidate();
  if (ready) setStatus('Source changed. Compiling shortly…');
  clearTimeout(debounce);
  debounce = setTimeout(compile, 300);
});
button.addEventListener('click', compile);
source.setOption('extraKeys', { 'Ctrl-Enter': compile, 'Cmd-Enter': compile });
document.querySelector('#examples').addEventListener('change', event => {
  source.setValue(`int main(void) {\n    return ${event.target.value};\n}`);
  compile();
});
document.querySelector('#keymap').addEventListener('change', event => {
  source.setOption('keyMap', event.target.value);
  source.focus();
});

async function copy(text, label) {
  try {
    await navigator.clipboard.writeText(text);
    setStatus(`${label} copied.`, 'ok');
  } catch {
    setStatus('Clipboard unavailable. Select the text and copy it manually.', 'error');
  }
}
document.querySelector('#copy-c').addEventListener('click', () => copy(source.getValue(), 'C program'));
copyClif.addEventListener('click', () => copy(output.getValue(), 'Cranelift IR'));
download.addEventListener('click', () => {
  if (!compiledBytes) return;
  const url = URL.createObjectURL(new Blob([compiledBytes], { type: 'application/wasm' }));
  const link = document.createElement('a');
  link.href = url;
  link.download = 'main.wasm';
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
});
for (const editor of [source, output]) {
  new ResizeObserver(() => editor.refresh()).observe(editor.getWrapperElement().parentElement);
}
