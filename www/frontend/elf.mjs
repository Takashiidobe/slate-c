const MAX_IMAGE = 4 * 1024 * 1024;
const align = (value, alignment) => Math.ceil(value / alignment) * alignment;

export function loadObject(bytes, arch, base, entryName) {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const number = offset => {
    const value = view.getBigUint64(offset, true);
    if (value > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error('ELF offset exceeds the supported range.');
    return Number(value);
  };
  const string = (offset, end) => {
    let stop = offset;
    while (stop < end && bytes[stop]) stop++;
    return new TextDecoder().decode(bytes.subarray(offset, stop));
  };
  if (bytes.length < 64 || view.getUint32(0, true) !== 0x464c457f || bytes[4] !== 2 || bytes[5] !== 1 || view.getUint16(16, true) !== 1) {
    throw new Error('Expected a little-endian ELF64 relocatable object.');
  }
  const machine = view.getUint16(18, true);
  if (machine !== (arch === 'x86_64' ? 62 : 183)) throw new Error('ELF architecture does not match the emulator.');
  const start = number(40);
  const stride = view.getUint16(58, true);
  const count = view.getUint16(60, true);
  if (stride < 64 || start + stride * count > bytes.length) throw new Error('Invalid ELF section table.');
  const sections = Array.from({ length: count }, (_, index) => {
    const at = start + index * stride;
    const section = { index, type: view.getUint32(at + 4, true), flags: number(at + 8),
      offset: number(at + 24), size: number(at + 32), link: view.getUint32(at + 40, true),
      info: view.getUint32(at + 44, true), alignment: number(at + 48) || 1, stride: number(at + 56) };
    if (section.type !== 8 && section.offset + section.size > bytes.length) throw new Error('ELF section exceeds the object.');
    return section;
  });
  let size = 0;
  for (const section of sections) {
    if (!(section.flags & 2)) continue;
    if (section.flags & 1024) throw new Error('Thread-local data is not supported.');
    if (section.alignment > MAX_IMAGE || (section.alignment & (section.alignment - 1))) throw new Error('Unsupported section alignment.');
    size = align(size, section.alignment);
    section.address = base + size;
    section.position = size;
    size += section.size;
    if (size > MAX_IMAGE) throw new Error('The bare-metal program exceeds 4 MiB.');
  }
  const tables = new Map();
  for (const table of sections.filter(section => section.type === 2)) {
    const strings = sections[table.link];
    if (!strings || table.stride !== 24) throw new Error('Invalid ELF symbol table.');
    tables.set(table.index, Array.from({ length: table.size / 24 }, (_, index) => {
      const at = table.offset + index * 24;
      const nameOffset = view.getUint32(at, true);
      if (nameOffset >= strings.size) throw new Error('Invalid ELF symbol name.');
      return { name: string(strings.offset + nameOffset, strings.offset + strings.size),
        section: view.getUint16(at + 6, true), value: number(at + 8) };
    }));
  }
  const relocations = [];
  for (const table of sections.filter(section => section.type === 4)) {
    const section = sections[table.info];
    if (section?.position === undefined) continue;
    const symbols = tables.get(table.link);
    if (!symbols || table.stride !== 24) throw new Error('Invalid ELF relocation table.');
    for (let offset = table.offset; offset < table.offset + table.size; offset += 24) {
      const info = view.getBigUint64(offset + 8, true);
      const symbol = symbols[Number(info >> 32n)];
      const relocation = { section, offset: number(offset), kind: Number(info & 0xffffffffn),
        addend: view.getBigInt64(offset + 16, true), symbol };
      if (!symbol || relocation.offset >= section.size) throw new Error('Invalid ELF relocation.');
      relocations.push(relocation);
    }
  }
  const got = new Map();
  for (const relocation of relocations) {
    const kinds = arch === 'x86_64' ? [9, 41, 42] : [311, 312];
    if (kinds.includes(relocation.kind) && !got.has(relocation.symbol)) {
      size = align(size, 8);
      got.set(relocation.symbol, size);
      size += 8;
    }
  }
  size = align(size, 16);
  if (size + 72 > MAX_IMAGE) throw new Error('The bare-metal program exceeds 4 MiB.');
  const image = new Uint8Array(size + 72);
  const output = new DataView(image.buffer);
  for (const section of sections) {
    if (section.position !== undefined && section.type !== 8) image.set(bytes.subarray(section.offset, section.offset + section.size), section.position);
  }
  const resolve = symbol => {
    if (symbol.section === 0) throw new Error(`Unresolved external symbol: ${symbol.name}. Bare-metal execution has no libc.`);
    if (symbol.section === 0xfff1) return BigInt(symbol.value);
    const section = sections[symbol.section];
    if (section?.address === undefined) throw new Error(`Unsupported symbol section: ${symbol.name}.`);
    return BigInt(section.address + symbol.value);
  };
  for (const [symbol, offset] of got) output.setBigUint64(offset, resolve(symbol), true);
  const signed = (value, bits) => {
    if (value < -(1n << BigInt(bits - 1)) || value >= 1n << BigInt(bits - 1)) throw new Error('ELF relocation is out of range.');
    return Number(value);
  };
  const patch = (at, mask, value) => output.setUint32(at, ((output.getUint32(at, true) & ~mask) | value) >>> 0, true);
  for (const relocation of relocations) {
    const { section, offset, kind, symbol, addend } = relocation;
    const at = section.position + offset;
    const position = BigInt(base + at);
    const address = resolve(symbol) + addend;
    const delta = address - position;
    if (arch === 'x86_64') {
      switch (kind) {
        case 0: break;
        case 1: output.setBigUint64(at, BigInt.asUintN(64, address), true); break;
        case 2: case 4: output.setInt32(at, signed(delta, 32), true); break;
        case 10:
          if (address < 0n || address > 0xffffffffn) throw new Error('Absolute relocation is out of range.');
          output.setUint32(at, Number(address), true); break;
        case 11: output.setInt32(at, signed(address, 32), true); break;
        case 9: case 41: case 42: output.setInt32(at, signed(BigInt(base + got.get(symbol)) + addend - position, 32), true); break;
        default: throw new Error(`Unsupported x86-64 ELF relocation ${kind} against ${symbol.name}.`);
      }
    } else {
      switch (kind) {
        case 0: break;
        case 257: output.setBigUint64(at, BigInt.asUintN(64, address), true); break;
        case 258: output.setUint32(at, Number(BigInt.asUintN(32, address)), true); break;
        case 260: output.setBigInt64(at, delta, true); break;
        case 261: output.setInt32(at, signed(delta, 32), true); break;
        case 282: case 283:
          if (delta % 4n) throw new Error('Misaligned branch relocation.');
          patch(at, 0x03ffffff, signed(delta / 4n, 26) & 0x03ffffff); break;
        case 273:
          if (delta % 4n) throw new Error('Misaligned literal relocation.');
          patch(at, 0x00ffffe0, (signed(delta / 4n, 19) & 0x7ffff) << 5); break;
        case 274: {
          const immediate = signed(delta, 21) & 0x1fffff;
          patch(at, 0x60ffffe0, ((immediate & 3) << 29) | ((immediate >> 2) << 5)); break;
        }
        case 275: case 276: case 311: {
          const target = kind === 311 ? BigInt(base + got.get(symbol)) + addend : address;
          const immediate = signed((target >> 12n) - (position >> 12n), 21) & 0x1fffff;
          patch(at, 0x60ffffe0, ((immediate & 3) << 29) | ((immediate >> 2) << 5)); break;
        }
        case 277: patch(at, 0x003ffc00, Number(address & 0xfffn) << 10); break;
        case 278: case 284: case 285: case 286: case 299: case 312: {
          const shift = { 278: 0, 284: 1, 285: 2, 286: 3, 299: 4, 312: 3 }[kind];
          const target = kind === 312 ? BigInt(base + got.get(symbol)) + addend : address;
          if (target % (1n << BigInt(shift))) throw new Error('Misaligned data relocation.');
          patch(at, 0x003ffc00, Number((target & 0xfffn) >> BigInt(shift)) << 10); break;
        }
        default: throw new Error(`Unsupported AArch64 ELF relocation ${kind} against ${symbol.name}.`);
      }
    }
  }
  let entry;
  for (const symbols of tables.values()) {
    const symbol = symbols.find(symbol => symbol.name === entryName && symbol.section !== 0);
    if (symbol) entry = Number(resolve(symbol));
  }
  if (entry === undefined) throw new Error('The compiler did not emit the runner entry point.');
  output.setBigUint64(size, 4n, true);
  return { image, entry, result: base + size };
}
