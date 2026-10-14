import hashlib
from pathlib import Path
import shutil
import subprocess
import tarfile
import urllib.request


ROOT = Path(__file__).resolve().parents[2]
VERSION = '0.0.3'
SHA256 = '78cbc9813f419c10640633bec920b157680175f597dcd801a2bc0c0a4afb0eec'
BUILD = ROOT / 'build/nixvm'
SOURCE = BUILD / f'nixvm-{VERSION}'
PACKAGE = BUILD / 'pkg'


def main():
    archive = BUILD / f'nixvm-{VERSION}.crate'
    BUILD.mkdir(parents=True, exist_ok=True)
    if not archive.exists():
        urllib.request.urlretrieve(
            f'https://crates.io/api/v1/crates/nixvm/{VERSION}/download', archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != SHA256:
        raise RuntimeError('nixvm crate checksum mismatch')
    if not SOURCE.exists():
        with tarfile.open(archive) as source:
            source.extractall(BUILD, filter='data')
    subprocess.run([
        'wasm-pack', 'build', str(SOURCE), '--target', 'web', '--release',
        '--out-dir', str(PACKAGE), '--no-typescript', '--no-pack',
        '--', '--no-default-features', '--features', 'wasm',
    ], check=True, cwd=ROOT)
    print(f'nixvm wasm runner ready: {PACKAGE}')


if __name__ == '__main__':
    main()
