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
const outputTabs = [...document.querySelectorAll('#output-tabs [role="tab"]')];
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
const instructionHelp = Object.fromEntries(['x86', 'arm'].map(arch => [arch,
  fetch(`instruction-help/${arch}.json`).then(response => response.json()).catch(() => ({}))]));
let activeAssemblyHints;
let activeAssemblyTitles;

function splitOperands(text) {
  const parts = [];
  let start = 0, depth = 0;
  for (let i = 0; i < text.length; i++) {
    if ('[('.includes(text[i])) depth++;
    else if ('])'.includes(text[i])) depth--;
    else if (text[i] === ',' && depth === 0) { parts.push(text.slice(start, i).trim()); start = i + 1; }
  }
  if (text.slice(start).trim()) parts.push(text.slice(start).trim());
  return parts;
}

function instructionRecord(hint, db) {
  const [mnemonic, ...tail] = hint.text.split(/\s+/);
  let key = mnemonic.toLowerCase();
  const operandsText = tail.join(' ');
  if (hint.arch === 'x86_64' && hint.syntax === 'att' && !db[key]) key = key.replace(/[bwlq]$/, '');
  const records = db[hint.arch === 'x86_64' ? 'x86' : 'arm']?.[key];
  if (!records) return null;
  const actual = splitOperands(operandsText);
  const score = record => {
    const syntax = record.syntax ?? '';
    const expected = splitOperands(syntax.slice(syntax.indexOf(' ') + 1));
    const optional = splitOperands(syntax.slice(syntax.indexOf(' ') + 1).replace(/[{}]/g, ''));
    let value = -Math.min(Math.abs(actual.length - expected.length), Math.abs(actual.length - optional.length)) * 4;
    if (hint.arch === 'x86_64') {
      for (let i = 0; i < Math.min(actual.length, expected.length); i++) {
        const form = expected[i].toLowerCase();
        const operand = actual[hint.syntax === 'att' ? actual.length - i - 1 : i].toLowerCase();
        const family = /%?(?:r(?:ax|bx|cx|dx|si|di|sp|bp|\d+)|e(?:ax|bx|cx|dx|si|di|sp|bp))\b/.test(operand) ? 'gpr'
          : /%?xmm\d+/.test(operand) ? 'xmm' : /%?ymm\d+/.test(operand) ? 'ymm' : /%?zmm\d+/.test(operand) ? 'zmm' : null;
        if (family) value += form.includes(family) ? 8 : -8;
        if (operand.startsWith('$')) value += /imm/.test(form) ? 5 : -2;
        if (/\[|\(/.test(operand)) value += /m(?:\d+)?|r\/m/.test(form) ? 2 : 0;
      }
      return value;
    }
    if (record.isa?.length && !record.isa.includes('A64')) return -Infinity;
    for (let i = 0; i < actual.length; i++) {
      const operand = actual[i], form = expected[i] ?? '';
      for (const [register, placeholder] of [[/[xy]\d+|\bsp\b/i, /<X/i], [/\bw\d+\b/i, /<W/i], [/\bv\d+\b/i, /<V/i], [/\bz\d+\b/i, /<Z/i], [/\bp\d+\b/i, /<P/i]]) {
        if (register.test(operand)) value += placeholder.test(form) ? 5 : -4;
      }
      if (/[bhdsq]\d+/i.test(operand) && /<[BHDSQ]/.test(form)) value += 4;
      if (operand.includes('[') === form.includes('[')) value++;
      if (operand.includes('#') === form.includes('#')) value++;
    }
    return value;
  };
  return [...records].sort((a, b) => score(b) - score(a))[0];
}

function describeInstruction(record, hint) {
  if (!record) return `Decoded instruction ${hint.text}.`;
  const text = hint.text.trim();
  const space = text.indexOf(' ');
  const actual = splitOperands(space < 0 ? '' : text.slice(space + 1));
  const template = splitOperands((record.syntax ?? '').replace(/^\S+\s*/, '').replace(/\s*\{.*$/, ''));
  const values = {};
  if (hint.arch === 'x86_64') {
    for (let i = 0; i < Math.min(actual.length, template.length); i++) {
      const index = hint.syntax === 'att' ? actual.length - i - 1 : i;
      const role = record.operands?.[index]?.name;
      if (role) values[role] = actual[i].replace(/^\*/, '').replace(/^%/, '').replace(/^\$/, '');
    }
    const clean = value => value?.replace(/^\*/, '').replace(/^%/, '').replace(/^\$/, '');
    const mnemonic = record.syntax?.split(' ')[0]?.toLowerCase();
    if (mnemonic === 'mov' && values.src && values.dest && /^[a-z]+\d*$/i.test(clean(values.src)) && /^[a-z]+\d*$/i.test(clean(values.dest))) {
      const special = { rsp: 'current stack pointer', esp: 'current stack pointer', sp: 'current stack pointer', rbp: 'current base pointer', ebp: 'current base pointer', bp: 'current base pointer' };
      const source = clean(values.src), destination = clean(values.dest);
      return `Copies ${special[source.toLowerCase()] ? `${special[source.toLowerCase()]} (${source})` : source} into ${special[destination.toLowerCase()] ? `${special[destination.toLowerCase()]} (${destination})` : destination}. No flags are affected.`;
    }
    let description = record.description ?? record.summary;
    if (values.src) description = description.replace(/\bsource operand\b/gi, `source (${values.src})`);
    if (values.dest) description = description.replace(/\bdestination operand\b/gi, `destination (${values.dest})`);
    if (actual.length === 1) {
      const operand = clean(values.src ?? values.dest ?? actual[0]);
      if (mnemonic === 'push') return `Pushes ${/^\s*\$/.test(actual[0]) ? operand : `the value in ${operand}`} onto the stack, then decrements rsp by 8 bytes and stores it at [rsp]. No flags are modified.`;
      if (mnemonic === 'pop') return `Pops the value at [rsp] into ${operand}, then increments rsp by 8 bytes. No flags are modified.`;
      description = description.replace(/\bthe operand\b/gi, operand).replace(/\boperand\b/gi, operand);
    }
    return `${record.name}. ${description}`;
  }
  for (let i = 0; i < Math.min(actual.length, template.length); i++) {
    const names = [...template[i].matchAll(/<([^>]+)>/g)].flatMap(match => match[1].split('|'));
    for (const name of names) {
      const family = /^([BHWXSDQVZP])/.exec(name)?.[1];
      const match = family ? new RegExp(`\\b${family}\\d+\\b|\\bSP\\b`, 'i').exec(actual[i]) : null;
      values[name] = match?.[0] ?? actual[i];
    }
  }
  let description = record.description ?? record.summary;
  for (const name of Object.keys(values).sort((a, b) => b.length - a.length)) {
    description = description.replace(new RegExp(`\\b${name}\\b`, 'g'), values[name]);
  }
  return `${record.name}. ${description}`;
}

function registerHelp(register, arch) {
  const name = register.toLowerCase();
  if (arch === 'x86_64') {
    const roles = { rsp: 'Stack pointer.', esp: 'Low 32 bits of the stack pointer.', rbp: 'Base/frame pointer register.', ebp: 'Low 32 bits of the base/frame pointer.', rip: 'Instruction pointer.', eip: 'Low 32 bits of the instruction pointer.', rax: 'Accumulator register.', eax: 'Low 32 bits of the accumulator.' };
    const argument = /^(rdi|edi|rsi|esi|rdx|edx|rcx|ecx|r8|r8d|r9|r9d)$/.exec(name);
    if (argument) {
      const index = { rdi: 1, edi: 1, rsi: 2, esi: 2, rdx: 3, edx: 3, rcx: 4, ecx: 4, r8: 5, r8d: 5, r9: 6, r9d: 6 }[name];
      return `System V integer argument register ${index}; caller-saved.`;
    }
    return roles[name] ?? (/^(xmm|ymm|zmm|mm|k|st)\d+$/.test(name) ? 'Vector, mask, or floating-point register.' : 'General-purpose register.');
  }
  if (name === 'sp' || name === 'wsp') return `Stack pointer ${register}. SP/WSP refer to the same stack-pointer state as the general-purpose register bank.`;
  if (name === 'xzr' || name === 'wzr') return 'Zero register. Reads as 0; writes are discarded.';
  if (name === 'x29' || name === 'w29' || name === 'fp') return 'Frame pointer (x29/fp). Commonly points to the current stack frame; callee-saved. w29 is its low 32-bit view.';
  if (name === 'x30' || name === 'w30' || name === 'lr') return 'Link register (x30/lr). A call instruction saves its return address here. w30 is its low 32-bit view.';
  if (name === 'ip0' || name === 'x16') return 'Intra-procedure-call temporary register. Linkers may use it for veneers and stubs.';
  if (name === 'ip1' || name === 'x17') return 'Intra-procedure-call temporary register. Linkers may use it for veneers and stubs.';
  const gp = /^[xw](\d+)$/.exec(name);
  if (gp) {
    const n = Number(gp[1]);
    if (n <= 7) return `${name[0] === 'w' ? `32-bit ${name} view of x${n}; writes clear the upper 32 bits. ` : '64-bit general-purpose register. '}Argument register ${n + 1}; x0 and x1 also carry integer results. Caller-saved.`;
    if (n === 8) return `${name[0] === 'w' ? '32-bit w8 view of x8; writes clear the upper 32 bits. ' : '64-bit general-purpose register. '}Register ${name} carries an indirect result address; caller-saved.`;
    if (n <= 17) return `${name[0] === 'w' ? `32-bit ${name} view of x${n}; writes clear the upper 32 bits. ` : '64-bit general-purpose register. '}Caller-saved temporary register.`;
    if (n === 18) return 'Platform register x18/w18; use depends on the operating system ABI.';
    if (n <= 29) return `${name[0] === 'w' ? `32-bit ${name} view of x${n}; writes clear the upper 32 bits. ` : '64-bit general-purpose register. '}Register ${name} is callee-saved; x29 is conventionally the frame pointer.`;
    return `${name[0] === 'w' ? `32-bit ${name} view of x${n}; writes clear the upper 32 bits. ` : '64-bit general-purpose register. '}Caller-saved temporary register.`;
  }
  const vector = /^([vbsdqhz])(\d+)$/.exec(name);
  if (vector) {
    const [, kind, number] = vector, n = Number(number);
    const view = { v: 'SIMD/floating-point', b: '8-bit SIMD/floating-point', h: '16-bit SIMD/floating-point', s: '32-bit SIMD/floating-point', d: '64-bit SIMD/floating-point', q: '128-bit SIMD/floating-point', z: 'scalable vector' }[kind];
    if (kind === 'z') return `Scalable vector register ${name}. z registers overlap the corresponding v registers and hold vectors whose size depends on the active SVE vector length.`;
    if (n <= 7) return `${view} register ${name}. The b/h/s/d/q/v names are views of the same SIMD register. Used for floating-point/vector arguments and results; caller-saved.`;
    if (n <= 15) return `${view} register ${name}. The b/h/s/d/q/v names are views of the same SIMD register. Callee-saved for its low 64 bits; wider values are caller-saved.`;
    return `${view} register ${name}. The b/h/s/d/q/v names are views of the same SIMD register; caller-saved.`;
  }
  if (/^p\d+$/.test(name)) {
    const n = Number(name.slice(1));
    return `Scalable vector predicate register ${name}. Controls which vector lanes participate in SVE operations.${n <= 3 ? ' p0-p3 can carry predicate arguments and results.' : ' Preservation across calls depends on whether the function uses scalable vector arguments.'}`;
  }
  if (name === 'nzcv') return 'Condition flags register: N (negative), Z (zero), C (carry), and V (overflow).';
  if (name === 'fpcr') return 'Floating-point control register. Controls floating-point behavior such as rounding and exception handling.';
  if (name === 'fpsr') return 'Floating-point status register. Holds cumulative floating-point exception status.';
  return `AArch64 register ${register}.`;
}

function operandTitle(token, hint) {
  const normalized = token.replace(/^%/, '').toLowerCase();
  const operand = hint.operands?.find(item => item.kind === 'register' && item.name.toLowerCase() === normalized
    || item.kind === 'memory' && [item.base, item.index].some(name => name && name.toLowerCase() === normalized));
  if (operand?.kind === 'register') return registerHelp(operand.name, hint.arch);
  if (operand?.kind === 'memory') {
    const name = operand.base || operand.index;
    if (name && name.toLowerCase() === normalized) return registerHelp(name, hint.arch);
  }
  const value = token.replace(/^[#$]/, '');
  const immediate = hint.operands?.find(item => item.kind === 'immediate' && Number(item.value) === Number(value));
  if (immediate) return `Immediate value ${Number(immediate.value)}.`;
  const displacement = hint.operands?.find(item => item.kind === 'memory' && Number(item.displacement) === Number(value));
  if (displacement) return `Memory address displacement ${Number(displacement.displacement)}.`;
  return null;
}

async function markAssemblyHints(field) {
  const hints = compilation?.[field.replace('Bytes', 'Hints')];
  if (!hints) return;
  const renderedText = output.getValue();
  const dbs = await Promise.all(Object.values(instructionHelp));
  if (output.getValue() !== renderedText) return;
  activeAssemblyHints = hints;
  activeAssemblyTitles = [];
  for (let lineNo = 0; lineNo < Math.min(hints.length, output.lineCount()); lineNo++) {
    const hint = hints[lineNo];
    if (!hint?.decoded) continue;
    const line = output.getLine(lineNo);
    const record = instructionRecord(hint, { x86: dbs[0], arm: dbs[1] });
    const description = describeInstruction(record, hint);
    activeAssemblyTitles[lineNo] = description;
    const start = line.search(/\S/);
    if (start < 0) continue;
    output.markText({ line: lineNo, ch: start }, { line: lineNo, ch: line.length }, { attributes: { title: description } });
    const registerNames = hint.operands.flatMap(item => item.kind === 'register' ? [item.name] : item.kind === 'memory' ? [item.base, item.index].filter(Boolean) : []);
    const names = registerNames.sort((a, b) => b.length - a.length).map(name => name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
    const registerPattern = names.length ? `(?<![a-zA-Z0-9_])%?(?:${names.join('|')})\\b|` : '';
    const pattern = new RegExp(`${registerPattern}(?<![a-zA-Z0-9_])(?:[$#])?[+-]?(?:0x[\\da-fA-F]+|\\d+)(?![a-zA-Z0-9_])`, 'gi');
    const instructionOffset = line.indexOf(hint.text, start);
    const operandOffset = instructionOffset + hint.text.indexOf(' ') + 1;
    const operandText = line.slice(operandOffset);
    for (const match of operandText.matchAll(pattern)) {
      const title = operandTitle(match[0], hint);
      if (title) output.markText({ line: lineNo, ch: operandOffset + match.index }, { line: lineNo, ch: operandOffset + match.index + match[0].length }, { className: 'operand-help', attributes: { title } });
    }
  }
}

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
  for (const tab of outputTabs) {
    const selected = tab.dataset.view === view.value;
    tab.setAttribute('aria-selected', String(selected));
    tab.tabIndex = selected ? 0 : -1;
  }
  shell.setAttribute('aria-labelledby', `${view.value}-tab`);
  syntax.hidden = view.value !== 'assembly' || !target.value.startsWith('x86_64');
  showBytesControl.hidden = !['assembly', 'elf'].includes(view.value);
  showBytesControl.lastChild.textContent = view.value === 'elf' ? ' Show meaning' : ' Show bytes';
  showBytes.checked = view.value === 'elf' ? showElfMeaning : showAssemblyBytes;
  const att = view.value === 'assembly' && !syntax.hidden && syntax.value === 'att';
  const field = view.value === 'assembly' ? `${att ? 'att' : ''}Assembly${showAssemblyBytes ? 'Bytes' : ''}`
    : view.value === 'elf' && showElfMeaning ? 'elfMeaning' : view.value;
  output.setOption('mode', view.value === 'clif' ? 'slate-clif' : view.value === 'assembly' ? 'slate-assembly'
    : view.value === 'ast' ? 'slate-sexpr' : view.value === 'tokens' ? 'slate-tokens'
      : view.value === 'slate_ir' ? 'slate-ir' : view.value === 'elf' ? 'slate-elf' : 'text/plain');
  output.setValue(compilation?.[field] ?? '');
  if (view.value === 'assembly') markAssemblyHints(field).catch(error => console.error(error));
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
for (const [index, tab] of outputTabs.entries()) {
  tab.addEventListener('click', () => { view.value = tab.dataset.view; showOutput(); save(); });
  tab.addEventListener('keydown', event => {
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const next = event.key === 'Home' ? 0 : event.key === 'End' ? outputTabs.length - 1
      : (index + (event.key === 'ArrowRight' ? 1 : outputTabs.length - 1)) % outputTabs.length;
    outputTabs[next].click();
    outputTabs[next].focus();
  });
}
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
output.getWrapperElement().addEventListener('mousemove', async event => {
  if (view.value !== 'assembly') return;
  const lineElement = event.target.closest('.CodeMirror-line');
  if (!lineElement) return;
  const position = output.coordsChar({ left: event.clientX, top: event.clientY }, 'window');
  const line = output.getLine(position.line);
  let hint = activeAssemblyHints?.[position.line];
  if (!hint?.decoded) {
    const instruction = line.trim().replace(/^(?:[0-9a-f]{2} ){1,}/i, '');
    const match = /^([a-z][a-z0-9.]*(?:\s+.*)?)$/i.exec(instruction);
    const arch = target.value.startsWith('x86_64') ? 'x86_64' : 'aarch64';
    if (!match || instruction.endsWith(':')) { lineElement.removeAttribute('title'); return; }
    hint = { text: match[1], arch, syntax: arch === 'x86_64' ? syntax.value : 'intel', operands: [], decoded: true };
  }
  const dbs = await Promise.all(Object.values(instructionHelp));
  const record = instructionRecord(hint, { x86: dbs[0], arm: dbs[1] });
  let title = activeAssemblyTitles?.[position.line] ?? describeInstruction(record, hint);
  const instructionOffset = line.indexOf(hint.text);
  const operandStart = instructionOffset + (hint.text.indexOf(' ') < 0 ? hint.text.length : hint.text.indexOf(' ') + 1);
  const operandText = line.slice(operandStart);
  const names = hint.operands.flatMap(item => item.kind === 'register' ? [item.name] : item.kind === 'memory' ? [item.base, item.index].filter(Boolean) : [])
    .sort((a, b) => b.length - a.length).map(name => name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
  const registerPattern = names.length ? `(?<![a-zA-Z0-9_])%?(?:${names.join('|')})\\b|` : '';
  const pattern = new RegExp(`${registerPattern}(?<![a-zA-Z0-9_])(?:[$#])?[+-]?(?:0x[\\da-fA-F]+|\\d+)(?![a-zA-Z0-9_])`, 'gi');
  for (const match of operandText.matchAll(pattern)) {
    const start = operandStart + match.index;
    if (position.ch >= start && position.ch <= start + match[0].length) title = operandTitle(match[0], hint) ?? title;
  }
  if (!hint.operands?.length) {
    const tokenPattern = /%?[a-z][a-z0-9]*|[$#]?[+-]?(?:0x[\da-f]+|\d+)/gi;
    for (const match of line.matchAll(tokenPattern)) {
      if (position.ch < match.index || position.ch > match.index + match[0].length) continue;
      if (/^(?:%?)(?:r(?:ax|bx|cx|dx|si|di|sp|bp|\d+[dwb]?)|e(?:ax|bx|cx|dx|si|di|sp|bp)|[xyz]mm\d+|[xw]\d+|[bhsdqv]\d+|[xw]zr|sp|wsp|lr|fp|p\d+)$/i.test(match[0])) title = registerHelp(match[0].replace(/^%/, ''), hint.arch);
      else if (/^[$#]/.test(match[0])) title = `Immediate value ${match[0].replace(/^[$#]/, '')}.`;
    }
  }
  if (lineElement.title !== title) lineElement.title = title;
});
