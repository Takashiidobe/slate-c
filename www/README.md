# Browser compiler

`frontend/` contains the site sources, `build.sh` builds the Rust compiler to
WebAssembly and assembles the site in `dist/`. Generated files are ignored.

From the project root:

```sh
rustup target add wasm32-unknown-unknown
./www/build.sh
python3 -m http.server --directory www/dist 8000
```

Open <http://localhost:8000>. The editor compiles automatically as you type.
Use Compile and run or Ctrl/Cmd+Enter to compile immediately. The right pane
shows CLIF; the execution bar shows the generated Wasm program’s return value.
Only the constant-return subset described in the root README is supported.

Compilation runs in a module Web Worker. The compiler Wasm loads locally;
there is no compilation server or CDN dependency. Standard, Vim, and Emacs
editor bindings are available. Successful compilations can be downloaded as
`main.wasm`. Changing the source invalidates the previous result and download.

```sh
node www/smoke.mjs
```

This checks the browser compiler bindings inside Node’s WebAssembly runtime,
including actual compilation, CLIF output, generated Wasm execution, and error
handling.

The layout and colors follow `../slate/www/frontend/index.html`. CodeMirror
5 assets were copied from Slate’s frontend, with their MIT license preserved
in `frontend/assets/CODEMIRROR-LICENSE`.
