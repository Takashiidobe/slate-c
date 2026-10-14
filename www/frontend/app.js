const source = CodeMirror.fromTextArea(document.querySelector('#c'), {
  mode: 'text/x-csrc', theme: 'material-darker', lineNumbers: true, indentUnit: 4, tabSize: 4,
});
const output = CodeMirror.fromTextArea(document.querySelector('#clif'), {
  mode: 'text/plain', theme: 'material-darker', lineNumbers: true, readOnly: true,
});
const target = document.querySelector('#target');
const cFlags = document.querySelector('#c-flags');
const clifFlags = document.querySelector('#cranelift-flags');
const button = document.querySelector('#compile');
const cancel = document.querySelector('#cancel');
const downloadObject = document.querySelector('#download-object');
const copyOutput = document.querySelector('#copy-clif');
const status = document.querySelector('#status');
const diagnostics = document.querySelector('#diagnostics');
const execution = document.querySelector('#execution');
const programOutput = document.querySelector('#program-output');
const stdout = document.querySelector('#stdout');
const stderr = document.querySelector('#stderr');
const shell = document.querySelector('#output-shell');
const view = document.querySelector('#output-view');
const syntax = document.querySelector('#asm-syntax');
const showBytesControl = document.querySelector('#show-bytes-control');
const showBytes = document.querySelector('#show-bytes');
const pending = new Map();
let worker;
let ready = false;
let requestId = 0;
let activeId;
let revision = 0;
let compilation;
let debounce;
let running = false;
let starting = false;
let schema = [];
let showAssemblyBytes = false;
let showElfMeaning = false;

function setStatus(text, kind = '') { status.textContent = text; status.className = kind; }

function flags(text, legacy = false) {
  const tokens = [];
  let token = '';
  let quote;
  let escaped = false;
  let started = false;
  for (const char of text) {
    if (escaped) { token += char; escaped = false; started = true; }
    else if (char === '\\' && quote !== "'") { escaped = true; started = true; }
    else if (quote) { if (char === quote) quote = undefined; else token += char; }
    else if (char === '"' || char === "'") { quote = char; started = true; }
    else if (legacy ? /\s/.test(char) : char === ',') { if (started && token.trim()) tokens.push(token.trim()); token = ''; started = false; }
    else { token += char; started = true; }
  }
  if (quote || escaped) throw new Error('Unterminated quote or escape in flags.');
  if (started && token.trim()) tokens.push(token.trim());
  return tokens;
}

function request() {
  return { source: source.getValue(), target: target.value, c_flags: flags(cFlags.value),
    cranelift_flags: flags(clifFlags.value) };
}

function compilerRequest() { return request(); }

function rpc(type, request, progress = true) {
  const id = ++requestId;
  if (progress) activeId = id;
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      pending.delete(id);
      reject(new Error(type === 'run' ? 'Execution timed out.' : 'Compilation timed out.'));
      if (type === 'run') worker.postMessage({ type: 'cancel' });
    }, type === 'run' ? 180000 : 60000);
    pending.set(id, { resolve, reject, timer });
    worker.postMessage({ id, type, request });
  });
}

const settingControls = {
  target, cFlags, clifFlags, view, syntax,
  keymap: document.querySelector('#keymap'),
  example: document.querySelector('#examples'),
  filter: document.querySelector('#flag-filter'),
};
const settingsPanel = document.querySelector('.settings');
const urlKeys = { cFlags: 'c-flags', clifFlags: 'clif-flags' };

function snapshot() {
  return { version: 2, source: source.getValue(), ...Object.fromEntries(
    Object.entries(settingControls).map(([name, control]) => [name, control.value])),
    settingsOpen: settingsPanel.open };
}

function save() {
  try { localStorage.setItem('slate-c', JSON.stringify(snapshot())); } catch {}
}

function restore(saved) {
  if (typeof saved.source === 'string') source.setValue(saved.source);
  for (const [name, control] of Object.entries(settingControls)) {
    let value = saved[name];
    if (typeof value !== 'string') continue;
    if (name === 'target') value = value.startsWith('x86_64-') ? 'x86_64-unknown-linux-musl' : value;
    if (saved.version !== 2 && (name === 'cFlags' || name === 'clifFlags')) {
      value = flags(value, true).map(flag => JSON.stringify(flag)).join(', ');
    }
    if (control.tagName !== 'SELECT' || [...control.options].some(option => option.value === value)) control.value = value;
  }
  if (typeof saved.settingsOpen === 'boolean') settingsPanel.open = saved.settingsOpen;
}

function invalidate() {
  revision++;
  compilation = undefined;
  downloadObject.disabled = true;
  copyOutput.disabled = true;
  shell.classList.add('stale');
  diagnostics.hidden = true;
  execution.textContent = 'Waiting';
  execution.className = '';
  programOutput.hidden = true;
  stdout.textContent = '';
  stderr.textContent = '';
  save();
}

function showOutput() {
  syntax.hidden = view.value !== 'assembly' || !target.value.startsWith('x86_64');
  showBytesControl.hidden = !['assembly', 'elf'].includes(view.value);
  showBytesControl.lastChild.textContent = view.value === 'elf' ? ' Show meaning' : ' Show bytes';
  showBytes.checked = view.value === 'elf' ? showElfMeaning : showAssemblyBytes;
  const att = view.value === 'assembly' && !syntax.hidden && syntax.value === 'att';
  const field = view.value === 'assembly' ? `${att ? 'att' : ''}Assembly${showAssemblyBytes ? 'Bytes' : ''}`
    : view.value === 'elf' && showElfMeaning ? 'elfMeaning' : view.value;
  output.setOption('mode', view.value === 'clif' ? 'slate-clif' : view.value === 'assembly' ? 'slate-assembly' : 'text/plain');
  output.setValue(compilation?.[field] ?? '');
  copyOutput.disabled = !compilation;
}

function fail(error) {
  diagnostics.textContent = String(error.message ?? error);
  diagnostics.hidden = false;
  setStatus('Failed.', 'error');
}

async function compile() {
  clearTimeout(debounce);
  if (!ready || running) return;
  const snapshot = revision;
  try {
    setStatus('Compiling…');
    const result = await rpc('compile', compilerRequest());
    if (snapshot !== revision) return;
    compilation = result;
    showOutput();
    shell.classList.remove('stale');
    diagnostics.hidden = true;
    downloadObject.disabled = false;
    execution.textContent = 'Ready';
    setStatus(`Compiled in ${Math.round(result.ms)} ms.`, 'ok');
  } catch (error) { if (snapshot === revision) fail(error); }
}

async function run() {
  clearTimeout(debounce);
  if (!ready || running || starting) return;
  starting = true;
  button.disabled = true;
  await compile();
  starting = false;
  if (!compilation) { button.disabled = !ready; return; }
  const snapshot = revision;
  running = true;
  button.disabled = true;
  cancel.disabled = false;
  execution.textContent = 'Waiting';
  try {
    const result = await rpc('run', request());
    if (snapshot !== revision) return;
    execution.textContent = result.error ? `Failed: ${result.error}` : `Exited with status ${result.exitCode}`;
    stdout.textContent = result.stdout;
    stderr.textContent = result.stderr;
    programOutput.hidden = false;
    execution.className = result.error ? 'error' : 'ok';
    setStatus(`${result.error ? 'Stopped' : 'Ran'} in ${Math.round(result.ms)} ms.`, result.error ? 'error' : 'ok');
  } catch (error) {
    if (snapshot === revision) { fail(error); execution.textContent = 'Waiting'; }
  } finally {
    running = false;
    button.disabled = !ready;
    cancel.disabled = true;
    if (snapshot !== revision) compile();
  }
}

function changed() {
  invalidate();
  clearTimeout(debounce);
  if (ready && !running) debounce = setTimeout(compile, 400);
}

function flagMap() { return new Map(flags(clifFlags.value).map(flag => { const [name, ...value] = flag.split('='); return [name, value.length ? value.join('=') : 'true']; })); }

function renderSchema() {
  const container = document.querySelector('#flag-settings');
  container.replaceChildren();
  let overrides;
  try { overrides = flagMap(); } catch { return; }
  const filter = document.querySelector('#flag-filter').value.toLowerCase();
  for (const flag of schema.filter(flag => !filter || `${flag.name} ${flag.description}`.toLowerCase().includes(filter))) {
    const label = document.createElement('label');
    label.title = `${flag.shared ? 'Shared' : 'Target'}: ${flag.description}`;
    const name = document.createElement('span');
    name.textContent = flag.name;
    label.append(name);
    const control = document.createElement(flag.kind === 'enum' ? 'select' : 'input');
    if (flag.kind === 'enum') {
      for (const value of flag.values) { const option = document.createElement('option'); option.value = value; option.textContent = value; control.append(option); }
      control.value = overrides.get(flag.name) ?? flag.default;
    } else if (flag.kind === 'bool' || flag.kind === 'preset') {
      control.type = 'checkbox'; control.checked = (overrides.get(flag.name) ?? flag.default) === 'true';
    } else { control.type = 'number'; control.value = overrides.get(flag.name) ?? flag.default ?? ''; }
    control.addEventListener('change', () => {
      const current = flagMap();
      const value = control.type === 'checkbox' ? String(control.checked) : control.value;
      if (value === flag.default || value === '' || (flag.kind === 'preset' && value === 'false')) current.delete(flag.name);
      else current.set(flag.name, value);
      clifFlags.value = [...current].map(([name, value]) => schema.find(flag => flag.name === name)?.kind === 'preset' ? name : `${name}=${value}`).join(', ');
      changed();
    });
    label.append(control);
    container.append(label);
  }
}

async function loadSchema() {
  const selected = target.value;
  try {
    const result = await rpc('schema', { target: selected }, false);
    if (selected !== target.value) return;
    schema = result;
    renderSchema();
  } catch (error) { fail(error); }
}

function startWorker() {
  worker = new Worker('slate-worker.js', { type: 'module' });
  worker.onmessage = ({ data }) => {
    if (data.type === 'ready') { ready = true; button.disabled = false; loadSchema(); compile(); return; }
    if (data.type === 'load-error') { fail(new Error(data.error)); return; }
    if (data.type === 'status') { if (data.id === activeId) setStatus(data.text); return; }
    const promise = pending.get(data.id);
    if (!promise) return;
    clearTimeout(promise.timer);
    pending.delete(data.id);
    if (data.ok) promise.resolve(data.result); else promise.reject(new Error(data.error));
  };
  worker.onerror = event => {
    ready = false;
    button.disabled = true;
    const error = new Error(`Compiler worker failed: ${event.message}`);
    for (const promise of pending.values()) { clearTimeout(promise.timer); promise.reject(error); }
    pending.clear();
    fail(error);
  };
}

const examples = {
  libc: '#include <stdio.h>\n#include <stdlib.h>\n\nint main(void) {\n    int *value = malloc(sizeof(int));\n    if (!value) return 1;\n    *value = 42;\n    printf("The answer is %d\\n", *value);\n    free(value);\n    return 0;\n}',
  files: '#include <stdio.h>\n\nint main(void) {\n    FILE *file = fopen("/tmp/example.txt", "w+");\n    if (!file) return 1;\n    fputs("Hello from a virtual file", file);\n    rewind(file);\n    char text[64];\n    if (!fgets(text, sizeof(text), file)) return 2;\n    puts(text);\n    fclose(file);\n    return 0;\n}',

  return42: 'int main(void) {\n    return 42;\n}',
  arithmetic: 'int bias = 5;\n\nint twice(int x) {\n    return x * 2;\n}\n\nint main(void) {\n    return twice(bias) + 32;\n}',
  loop: 'int main(void) {\n    int sum = 0;\n    for (int i = 0; i < 7; i++) {\n        sum += i * 2;\n    }\n    return sum;\n}',
  strings: `char word[] = "slate";\n\nint main(void) {\n    const char *message = "Hello";\n    return word[0] - 's' + message[0] - 'H' + 42;\n}`,
  floats: 'double average(double x, double y) {\n    return (x + y) / 2.0;\n}\n\nint main(void) {\n    return (int)average(40.0, 44.0);\n}',
};

try { restore(JSON.parse(localStorage.getItem('slate-c') ?? '{}') ?? {}); } catch {}
const shared = new URL(location.href).searchParams;
const sharedSettings = { version: 2 };
for (const name of ['source', ...Object.keys(settingControls)]) {
  const value = shared.get(urlKeys[name] ?? name);
  if (value !== null) sharedSettings[name] = value;
}
if (shared.has('settingsOpen')) sharedSettings.settingsOpen = shared.get('settingsOpen') === 'true';
restore(sharedSettings);
source.setOption('keyMap', settingControls.keymap.value);
showOutput();
save();
source.on('change', changed);
cFlags.addEventListener('input', changed);
clifFlags.addEventListener('input', () => { changed(); renderSchema(); });
target.addEventListener('change', () => { clifFlags.value = 'opt_level=speed'; showOutput(); changed(); if (ready) loadSchema(); });
view.addEventListener('change', () => { showOutput(); save(); });
syntax.addEventListener('change', () => { showOutput(); save(); });
showBytes.addEventListener('change', () => {
  if (view.value === 'elf') showElfMeaning = showBytes.checked;
  else showAssemblyBytes = showBytes.checked;
  showOutput();
});
document.querySelector('#flag-filter').addEventListener('input', () => { renderSchema(); save(); });
button.addEventListener('click', run);
cancel.addEventListener('click', () => { worker.postMessage({ type: 'cancel' }); setStatus('Cancelling…'); });
source.setOption('extraKeys', { 'Ctrl-Enter': run, 'Cmd-Enter': run });
document.querySelector('#examples').addEventListener('change', event => { source.setValue(examples[event.target.value]); save(); });
document.querySelector('#keymap').addEventListener('change', event => { source.setOption('keyMap', event.target.value); source.focus(); save(); });
async function copy(text) {
  try { await navigator.clipboard.writeText(text); setStatus('Copied.', 'ok'); }
  catch { setStatus('Select the text and copy it manually.', 'error'); }
}
document.querySelector('#copy-c').addEventListener('click', () => copy(source.getValue()));
copyOutput.addEventListener('click', () => copy(output.getValue()));
settingsPanel.addEventListener('toggle', save);
document.querySelector('#share').addEventListener('click', () => {
  try {
    request();
    save();
    const url = new URL(location.href);
    for (const [name, value] of Object.entries(snapshot())) {
      if (name !== 'version') url.searchParams.set(urlKeys[name] ?? name, String(value));
    }
    history.replaceState(null, '', url);
    copy(url.href);
  } catch (error) { fail(error); }
});
function saveBytes(bytes, filename) {
  if (!bytes) return;
  const url = URL.createObjectURL(new Blob([bytes], { type: 'application/octet-stream' }));
  const link = document.createElement('a'); link.href = url; link.download = filename; link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
downloadObject.addEventListener('click', () => saveBytes(compilation?.object, 'main.o'));
for (const editor of [source, output]) new ResizeObserver(() => editor.refresh()).observe(editor.getWrapperElement().parentElement);
startWorker();
