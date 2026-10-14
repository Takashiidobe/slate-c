import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile


ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT))
from runtime.softfloat.build_runtime import build as build_softfloat
OUT = ROOT / 'build/blink-runtime'
VARARGS = ['printf', 'fprintf', 'sprintf', 'snprintf', 'scanf', 'fscanf', 'sscanf',
           'dprintf', 'asprintf', 'open', 'openat', 'fcntl', 'ioctl']
FILES = [
    'lib/ld-musl-x86_64.so.1', 'lib/libc.musl-x86_64.so.1', 'usr/bin/ld.bfd',
    'usr/lib/libbfd-2.45.1.so', 'usr/lib/libctf.so.0', 'usr/lib/libjansson.so.4',
    'usr/lib/libz.so.1', 'usr/lib/libzstd.so.1', 'usr/lib/libsframe.so.2',
    'usr/lib/crt1.o', 'usr/lib/crti.o', 'usr/lib/crtn.o', 'usr/lib/libc.a',
    'usr/lib/libgcc.a', 'usr/lib/libslate.a',
    'usr/share/licenses/slate-softfloat/COPYING.txt',
]


def main():
    sources = json.loads((ROOT / 'www/runtime/sources.json').read_text())
    root = OUT / 'root'
    root.mkdir(parents=True, exist_ok=True)
    downloads = ROOT / 'build/runtime/downloads/x86_64'
    downloads.mkdir(parents=True, exist_ok=True)
    for package in sources['packages']:
        path = downloads / package['url'].rsplit('/', 1)[1]
        if not path.exists():
            subprocess.run(['curl', '-fL', '--retry', '3', package['url'], '-o', str(path)], check=True)
        if hashlib.sha256(path.read_bytes()).hexdigest() != package['sha256']:
            raise RuntimeError(f'checksum mismatch: {path}')
        with tarfile.open(path, ignore_zeros=True) as archive:
            if path.name.startswith('gcc-'):
                member = next(member for member in archive if member.name.endswith('/libgcc.a'))
                library = root / 'usr/lib/libgcc.a'
                library.parent.mkdir(parents=True, exist_ok=True)
                contents = archive.extractfile(member)
                if contents is None:
                    raise RuntimeError('libgcc archive member has no contents')
                library.write_bytes(contents.read())
            else:
                members = [member for member in archive if member.name.startswith(('lib/', 'usr/lib/', 'usr/include/', 'usr/bin/ld'))]
                archive.extractall(root, members=members, filter='data')
    assembly = ['.text']
    for name in VARARGS:
        assembly.extend([f'.global __slate_vararg_{name}', f'__slate_vararg_{name}:',
                         'mov $8, %eax', f'jmp {name}'])
    assembly.append('.section .note.GNU-stack,"",@progbits')
    (OUT / 'varargs.S').write_text('\n'.join(assembly) + '\n')
    subprocess.run(['clang', '--target=x86_64-unknown-linux-musl', '-c',
                    str(OUT / 'varargs.S'), '-o', str(OUT / 'varargs.o')], check=True)
    softfloat = build_softfloat(OUT / 'softfloat', target='x86_64-unknown-linux-musl', sysroot=root)
    runtime = root / 'usr/lib/libslate.a'
    runtime.unlink(missing_ok=True)
    subprocess.run(['llvm-ar', 'rcs', str(runtime), str(OUT / 'varargs.o'),
                    *(str(path) for path in softfloat)], check=True)
    license_path = root / 'usr/share/licenses/slate-softfloat/COPYING.txt'
    license_path.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(ROOT / 'runtime/softfloat/vendor/COPYING.txt', license_path)
    with tarfile.open(OUT / 'runtime.tar.gz', 'w:gz', format=tarfile.USTAR_FORMAT, dereference=True) as archive:
        for name in FILES:
            archive.add(root / name, arcname=name)
    with tarfile.open(OUT / 'headers.tar.gz', 'w:gz', format=tarfile.USTAR_FORMAT, dereference=True) as archive:
        archive.add(root / 'usr/include', arcname='usr/include')
    shutil.copyfile(ROOT / 'www/runtime/sources.json', OUT / 'SOURCES.json')
    print(f'Blink linker and musl runtime ready: {OUT}')


if __name__ == '__main__':
    main()
