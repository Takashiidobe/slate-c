import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import os
from pathlib import Path
import shutil
import subprocess


ROOT = Path(__file__).resolve().parent


def build(output, cc='clang', ar='llvm-ar', target=None, sysroot=None):
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    flags = ['-std=c11', '-O2', '-fPIC', '-ffreestanding', '-Werror=implicit-function-declaration']
    if target:
        flags.append(f'--target={target}')
    if sysroot:
        flags.append(f'--sysroot={sysroot}')
    for directory in [ROOT, ROOT / 'vendor/source/include', ROOT / 'vendor/source/8086-SSE']:
        flags.extend(['-I', str(directory)])
    flags.extend(f'-D{definition}' for definition in (ROOT / 'defines.txt').read_text().splitlines())
    sources = (ROOT / 'sources.txt').read_text().splitlines() + ['helpers.c']

    def compile_source(source):
        name = hashlib.sha256(source.encode()).hexdigest()[:8]
        object_path = output / f'{name}-{Path(source).stem}.o'
        subprocess.run([cc, *flags, '-c', str(ROOT / source), '-o', str(object_path)], check=True)
        return object_path

    with ThreadPoolExecutor(max_workers=min(os.cpu_count() or 1, 8)) as executor:
        objects = list(executor.map(compile_source, sources))
    archive = output / 'libslate_f80.a'
    archive.unlink(missing_ok=True)
    subprocess.run([ar, 'rcs', str(archive), *(str(path) for path in objects)], check=True)
    shutil.copyfile(ROOT / 'vendor/COPYING.txt', output / 'SOFTFLOAT-LICENSE.txt')
    return objects


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--output', required=True)
    parser.add_argument('--cc', default='clang')
    parser.add_argument('--ar', default='llvm-ar')
    parser.add_argument('--target')
    parser.add_argument('--sysroot')
    args = parser.parse_args()
    build(args.output, args.cc, args.ar, args.target, args.sysroot)
