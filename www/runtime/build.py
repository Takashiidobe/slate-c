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
ARCHIVE_SOURCES = ROOT / 'www/runtime/aarch64-sources.json'


def download(package, downloads):
    path = downloads / package['url'].rsplit('/', 1)[1]
    if not path.exists():
        subprocess.run(['curl', '-fL', '--retry', '3', package['url'], '-o', str(path)], check=True)
    if hashlib.sha256(path.read_bytes()).hexdigest() != package['sha256']:
        raise RuntimeError(f'checksum mismatch: {path}')
    return path


def extract(package, path, root):
    with tarfile.open(path, 'r:*') as archive:
        members = [member for member in archive if member.name.startswith(
            ('lib/', 'usr/lib/', 'usr/include/', 'usr/bin/ld', 'usr/aarch64-alpine-linux-musl/'))]
        archive.extractall(root, members=members, filter='data')


def main():
    sources = json.loads((ROOT / 'www/runtime/sources.json').read_text())
    root = OUT / 'root'
    root.mkdir(parents=True, exist_ok=True)
    downloads = ROOT / 'build/runtime/downloads/x86_64'
    downloads.mkdir(parents=True, exist_ok=True)
    for package in sources['packages']:
        path = download(package, downloads)
        if package['source'] == 'gcc':
            with tarfile.open(path, 'r:*') as archive:
                member = next(member for member in archive if member.name.endswith('/libgcc.a'))
                library = root / 'usr/lib/libgcc.a'
                library.parent.mkdir(parents=True, exist_ok=True)
                contents = archive.extractfile(member)
                if contents is None:
                    raise RuntimeError('libgcc archive member has no contents')
                library.write_bytes(contents.read())
        else:
            extract(package, path, root)
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

    build_aarch64(OUT, downloads.parent)


def build_aarch64(out, downloads_root):
    sources = json.loads(ARCHIVE_SOURCES.read_text())
    target_root = out / 'aarch64-sysroot'
    shutil.rmtree(target_root, ignore_errors=True)
    target_root.mkdir(parents=True, exist_ok=True)
    downloads = downloads_root / 'aarch64'
    downloads.mkdir(parents=True, exist_ok=True)
    cross_binutils = None
    for package in sources['packages']:
        path = download(package, downloads)
        if package['source'] == 'binutils':
            cross_binutils = path
        elif package['source'] == 'compiler-rt':
            with tarfile.open(path, 'r:*') as archive:
                member = next(member for member in archive if member.name.endswith(
                    '/aarch64-alpine-linux-musl/libclang_rt.builtins-aarch64.a'))
                library = target_root / 'usr/lib/libgcc.a'
                library.parent.mkdir(parents=True, exist_ok=True)
                contents = archive.extractfile(member)
                if contents is None:
                    raise RuntimeError('compiler-rt archive member has no contents')
                library.write_bytes(contents.read())
        elif package['source'] == 'gcc':
            with tarfile.open(path, 'r:*') as archive:
                member = next(member for member in archive if member.name.endswith('/libgcc.a'))
                library = target_root / 'usr/lib/libgcc.a'
                library.parent.mkdir(parents=True, exist_ok=True)
                contents = archive.extractfile(member)
                if contents is None:
                    raise RuntimeError('libgcc archive member has no contents')
                library.write_bytes(contents.read())
        else:
            extract(package, path, target_root)
    if cross_binutils is None:
        raise RuntimeError('missing AArch64 cross-binutils package')

    cross_root = out / 'aarch64-cross-binutils'
    shutil.rmtree(cross_root, ignore_errors=True)
    cross_root.mkdir(parents=True, exist_ok=True)
    cross_package = next(package for package in sources['packages'] if package['source'] == 'binutils')
    extract(cross_package, cross_binutils, cross_root)

    varargs = out / 'aarch64-varargs.S'
    names = ['printf', 'fprintf', 'sprintf', 'snprintf', 'scanf', 'fscanf', 'sscanf', 'dprintf',
             'asprintf', 'open', 'openat', 'fcntl', 'ioctl']
    lines = ['.text']
    for name in names:
        lines.extend([f'.global __slate_vararg_{name}', f'__slate_vararg_{name}:', f'  b {name}'])
    lines.append('.section .note.GNU-stack,"",%progbits')
    varargs.write_text('\n'.join(lines) + '\n')
    wrapper = out / 'aarch64-varargs.o'
    subprocess.run(['clang', '--target=aarch64-alpine-linux-musl', '-c', str(varargs), '-o', str(wrapper)], check=True)
    subprocess.run(['llvm-ar', 'rcs', str(target_root / 'usr/lib/libslate.a'), str(wrapper)], check=True)

    runtime = out / 'aarch64-runtime-root'
    shutil.rmtree(runtime, ignore_errors=True)
    shutil.copytree(root := out / 'root', runtime, symlinks=True)
    shutil.copytree(cross_root / 'usr/aarch64-alpine-linux-musl',
                    runtime / 'usr/aarch64-alpine-linux-musl', symlinks=True)
    sysroot = runtime / 'sysroots/aarch64'
    shutil.copytree(target_root, sysroot, symlinks=True)
    with tarfile.open(out / 'aarch64-runtime.tar.gz', 'w:gz', format=tarfile.USTAR_FORMAT,
                      dereference=True) as archive:
        for name in ['lib', 'usr/lib', 'usr/bin/ld', 'usr/bin/ld.bfd']:
            path = runtime / name
            if path.exists():
                archive.add(path, arcname=name)
        archive.add(runtime / 'usr/aarch64-alpine-linux-musl', arcname='usr/aarch64-alpine-linux-musl')
        for path in sysroot.rglob('*'):
            if path.is_file() and not path.is_symlink():
                archive.add(path, arcname=path.relative_to(runtime))
    with tarfile.open(out / 'aarch64-headers.tar.gz', 'w:gz', format=tarfile.USTAR_FORMAT, dereference=True) as archive:
        archive.add(target_root / 'usr/include', arcname='usr/include')
    shutil.copyfile(ARCHIVE_SOURCES, out / 'AARCH64-SOURCES.json')
    print(f'AArch64 musl sysroot and cross linker ready: {out}')


if __name__ == '__main__':
    main()
