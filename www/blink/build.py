import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / 'build/blink-src'
OUT = ROOT / 'build/blink-demo'
REVISION = 'f006a4fc6f9b8de9272504fdff0dbbe5ce5dc580'
LINK_FLAGS = [
    '-O2', '-sALLOW_MEMORY_GROWTH=1', '-sMODULARIZE=1', '-sEXPORT_ES6=1',
    '-sENVIRONMENT=web,worker,node', '-sFORCE_FILESYSTEM=1',
    '-sEXPORTED_RUNTIME_METHODS=FS,callMain', '-sINVOKE_RUN=0', '-sEXIT_RUNTIME=1',
]


def run(args, cwd=ROOT):
    subprocess.run(args, cwd=cwd, check=True)


def build_runtime():
    for command in ['git', 'make', 'emcc', 'emar']:
        if not shutil.which(command):
            raise RuntimeError(f'missing required command: {command}')
    SOURCE.parent.mkdir(parents=True, exist_ok=True)
    if not SOURCE.exists():
        run(['git', 'clone', 'https://github.com/jart/blink.git', str(SOURCE)])
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=SOURCE, text=True).strip()
    if revision != REVISION:
        run(['git', 'fetch', 'origin', REVISION], SOURCE)
        run(['git', 'checkout', '--detach', REVISION], SOURCE)
    stamp = json.dumps([REVISION, LINK_FLAGS, subprocess.check_output(['emcc', '--version'], text=True)])
    if (OUT / 'build-stamp.json').exists() and (OUT / 'build-stamp.json').read_text() == stamp and all((OUT / name).exists() for name in ['blink.mjs', 'blink.wasm', 'BLINK-LICENSE']):
        return
    run(['./configure', 'CC=emcc', 'AR=emar', 'MODE=rel', '--disable-jit',
         '--disable-threads', '--disable-fork', '--disable-sockets', '--disable-metal',
         'CFLAGS=-O2', f'LDFLAGS={" ".join(LINK_FLAGS)}'], SOURCE)
    run(['make', f'-j{min(os.cpu_count() or 1, 8)}', 'o/rel/blink/blink'], SOURCE)
    OUT.mkdir(parents=True, exist_ok=True)
    run(['emcc', *LINK_FLAGS, 'o/rel/blink/blink.o', 'o/rel/blink/blink.a',
         '-o', str(OUT / 'blink.mjs')], SOURCE)
    shutil.copyfile(SOURCE / 'LICENSE', OUT / 'BLINK-LICENSE')
    (OUT / 'build-stamp.json').write_text(stamp)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--runtime-only', action='store_true')
    args = parser.parse_args()
    build_runtime()
    if args.runtime_only:
        return
    run(['musl-gcc', '-static', '-O2', str(ROOT / 'www/blink/example.c'),
         '-o', str(OUT / 'example')])
    for name in ['index.html', 'app.js', 'worker.js']:
        shutil.copyfile(ROOT / 'www/blink' / name, OUT / name)
    shutil.copyfile(SOURCE / 'LICENSE', OUT / 'BLINK-LICENSE')
    print(f'Blink demo ready: {OUT}')


if __name__ == '__main__':
    main()
