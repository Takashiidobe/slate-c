import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile


ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'www/dist'
EXPLORER = Path(os.environ.get('CLIF_EXPLORER', ROOT.parent / 'clif-explorer'))
SLATE = ROOT.parent / 'slate'


def emit(source, stem, extension):
    digest = hashlib.sha256(source.read_bytes()).hexdigest()[:12]
    destination = OUT / f'{stem}.{digest}.{extension}'
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)
    return str(destination.relative_to(OUT))


def main():
    subprocess.run(['cargo', 'build', '--release', '--target', 'wasm32-wasip1',
                    '--no-default-features', '--features', 'browser', '--bin', 'slate-c-browser'], cwd=ROOT, check=True)
    subprocess.run(['python3', str(ROOT / 'www/blink/build.py'), '--runtime-only'], check=True)
    subprocess.run(['python3', str(ROOT / 'www/runtime/build.py')], check=True)
    blink = ROOT / 'build/blink-demo'
    runtime = ROOT / 'build/blink-runtime'
    if OUT.exists():
        shutil.rmtree(OUT)
    shutil.copytree(ROOT / 'www/frontend', OUT)
    shutil.copytree(SLATE / 'www/frontend/vendor/browser_wasi_shim', OUT / 'vendor/browser_wasi_shim')
    compiler = emit(ROOT / 'target/wasm32-wasip1/release/slate-c-browser.wasm', 'compiler', 'wasm')
    capstone = EXPLORER / 'build/capstone-wasm/out'
    manifest = {'compiler': compiler, 'targets': {}, 'capstone': {
        'js': emit(capstone / 'capstone.mjs', 'vendor/capstone', 'mjs'),
        'wasm': emit(capstone / 'capstone.wasm', 'vendor/capstone', 'wasm')}}
    header_root = Path(os.environ.get('SLATE_HEADERS', Path.home() / '.local/share/slate'))
    packed_root = ROOT / 'build/headers'
    packed_root.mkdir(parents=True, exist_ok=True)
    manifest['targets']['x86_64-unknown-linux-musl'] = {
        'arch': 'x86_64',
        'headers': emit(runtime / 'headers.tar.gz', 'headers/x86_64-musl', 'tar.gz'),
    }
    manifest['blink'] = {
        'js': emit(blink / 'blink.mjs', 'vendor/blink', 'mjs'),
        'wasm': emit(blink / 'blink.wasm', 'vendor/blink', 'wasm'),
        'runtime': emit(runtime / 'runtime.tar.gz', 'runtime/musl-linker', 'tar.gz'),
    }
    profiles = sorted((header_root / 'compiler-headers').glob('clang-*'))
    if not profiles:
        raise RuntimeError('missing Slate Clang builtin headers; set SLATE_HEADERS')
    profile = max(profiles, key=lambda path: tuple(int(part) for part in path.name.removeprefix('clang-').split('.')))
    packed = packed_root / 'compiler-headers.tar.gz'
    with tarfile.open(packed, 'w:gz', format=tarfile.USTAR_FORMAT, dereference=True) as archive:
        archive.add(profile, arcname=profile.name)
    manifest['compilerHeaders'] = emit(packed, 'headers/clang', 'tar.gz')
    (OUT / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    licenses = OUT / 'licenses'
    licenses.mkdir()
    shutil.copyfile(blink / 'BLINK-LICENSE', licenses / 'BLINK-LICENSE')
    shutil.copyfile(runtime / 'SOURCES.json', licenses / 'RUNTIME-SOURCES.json')
    shutil.copyfile(ROOT / 'runtime/softfloat/vendor/COPYING.txt', licenses / 'SOFTFLOAT-LICENSE.txt')
    shutil.copyfile(ROOT / 'runtime/softfloat/SOURCE.json', licenses / 'SOFTFLOAT-SOURCE.json')
    shutil.copytree(ROOT / 'www/runtime/licenses', licenses / 'runtime')
    shutil.copyfile(capstone / 'CAPSTONE-LICENSE', licenses / 'CAPSTONE-LICENSE')
    shutil.copyfile(EXPLORER / 'UPDATING.md', licenses / 'CAPSTONE-SOURCES.md')
    print(f'Browser build ready: {OUT}')


if __name__ == '__main__':
    main()
