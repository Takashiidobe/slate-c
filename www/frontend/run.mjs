import { loadObject } from './elf.mjs';
import { STUBS } from './stubs.mjs';
import { GuestTimeout, runGuest } from './qemu.mjs';

const PAGE = 4096;

function stub(arch) {
  const { size, chunks } = STUBS[arch];
  const bytes = new Uint8Array(size);
  for (const [offset, encoded] of chunks) bytes.set(Uint8Array.from(atob(encoded), char => char.charCodeAt(0)), offset);
  return bytes;
}

export async function runProgram(compilation, arch, load, signal) {
  const base = arch === 'x86_64' ? 0x101000 : 0x40081000;
  const { image, entry, result } = loadObject(compilation.object, arch, base, compilation.entry);
  const argumentsBlock = new Uint8Array(24);
  const view = new DataView(argumentsBlock.buffer);
  [entry, result, result].forEach((value, index) => view.setBigUint64(index * 8, BigInt(value), true));
  const files = {};
  let args;
  if (arch === 'x86_64') {
    const blob = new Uint8Array(PAGE + image.length);
    blob.set(argumentsBlock);
    blob.set(image, PAGE);
    files['/stub.bin'] = stub(arch);
    files['/blob.bin'] = blob;
    args = ['-machine', 'pc', '-cpu', 'max', '-bios', '/stub.bin',
      '-device', 'loader,file=/blob.bin,addr=0x100000,force-raw=on'];
  } else {
    const blob = new Uint8Array(PAGE + image.length);
    blob.set(stub(arch));
    blob.set(argumentsBlock, 0x100);
    blob.set(image, PAGE);
    files['/image.bin'] = blob;
    args = ['-machine', 'virt', '-cpu', 'max', '-kernel', '/image.bin'];
  }
  args.push('-serial', 'file:/serial.bin', '-display', 'none', '-monitor', 'none',
    '-nodefaults', '-no-reboot', '-m', '16', '-accel', 'tcg,tb-size=16');
  try {
    const record = await runGuest({ load, files, args, resultBytes: 4, timeoutMs: 5000, signal });
    const view = new DataView(record.buffer, record.byteOffset, record.byteLength);
    if (record[0] === 82) return { status: 'ok', value: view.getInt32(9, true) };
    const pc = view.getBigUint64(1, true);
    const cause = view.getBigUint64(9, true);
    return { status: 'trap', error: `Guest fault at 0x${pc.toString(16)} (cause 0x${cause.toString(16)}).` };
  } catch (error) {
    if (error instanceof GuestTimeout) return { status: 'timeout', error: 'Program exceeded the five-second execution limit.' };
    throw error;
  }
}
