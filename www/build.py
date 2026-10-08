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
    qemu = EXPLORER / 'build/qemu-wasm/out'
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
    for arch in ['x86_64', 'aarch64']:
        triple = f'{arch}-unknown-linux-gnu'
        sysroot = header_root / 'sysroots' / triple
        if not sysroot.exists():
            raise RuntimeError(f'missing Slate sysroot {triple}; set SLATE_HEADERS')
        packed_headers = packed_root / f'{arch}.tar.gz'
        with tarfile.open(packed_headers, 'w:gz', format=tarfile.USTAR_FORMAT, dereference=True) as archive:
            for name in ['usr/include', 'include', 'SDK/usr/include', f'usr/{arch}-linux-gnu/include', 'licenses']:
                path = sysroot / name
                if path.exists(): archive.add(path, arcname=name)
        manifest['targets'][triple] = {
            'arch': arch,
            'headers': emit(packed_headers, f'headers/{arch}', 'tar.gz'),
            'qemu': {'js': emit(qemu / f'qemu-system-{arch}.js', f'vendor/qemu-system-{arch}', 'js'),
                     'wasm': emit(qemu / f'qemu-system-{arch}.wasm', f'vendor/qemu-system-{arch}', 'wasm')},
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
    shutil.copyfile(qemu / 'QEMU-LICENSE', licenses / 'QEMU-LICENSE')
    shutil.copyfile(capstone / 'CAPSTONE-LICENSE', licenses / 'CAPSTONE-LICENSE')
    shutil.copyfile(EXPLORER / 'UPDATING.md', licenses / 'EMULATOR-SOURCES.md')
    print(f'Browser build ready: {OUT}')


if __name__ == '__main__':
    main()
