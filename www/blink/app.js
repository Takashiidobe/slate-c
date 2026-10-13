const output = document.querySelector('#output');
const status = document.querySelector('#status');
const stop = document.querySelector('#stop');
const example = document.querySelector('#example');
const file = document.querySelector('#file');
let worker;
let timer;
let started;

function finish(text) {
  clearTimeout(timer);
  worker?.terminate();
  worker = undefined;
  stop.disabled = true;
  example.disabled = false;
  file.disabled = false;
  status.textContent = text;
}

function run(executable) {
  finish('Starting…');
  output.textContent = '';
  example.disabled = true;
  file.disabled = true;
  stop.disabled = false;
  started = performance.now();
  worker = new Worker('worker.js', { type: 'module' });
  worker.onmessage = ({ data }) => {
    if (data.type === 'output') output.textContent += `${data.text}\n`;
    else if (data.type === 'exit') finish(`Exited with status ${data.code} in ${Math.round(performance.now() - started)} ms`);
    else if (data.type === 'error') finish(data.error);
  };
  worker.onerror = event => finish(event.message);
  timer = setTimeout(() => finish('Execution timed out.'), 10000);
  worker.postMessage({ executable }, [executable]);
}

example.addEventListener('click', async () => {
  try {
    const response = await fetch('example');
    if (!response.ok) throw new Error('Could not load example.');
    run(await response.arrayBuffer());
  } catch (error) { finish(error.message); }
});
file.addEventListener('change', async () => {
  try { if (file.files[0]) run(await file.files[0].arrayBuffer()); }
  catch (error) { finish(error.message); }
});
stop.addEventListener('click', () => finish('Execution cancelled.'));
