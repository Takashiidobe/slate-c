const hex = bytes => Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join(' ');
const numberHex = value => `0x${value.toString(16)}`;
const typeNames = new Map([[0, 'NULL'], [1, 'PROGBITS'], [2, 'SYMTAB'], [3, 'STRTAB'],
  [4, 'RELA'], [5, 'HASH'], [6, 'DYNAMIC'], [7, 'NOTE'], [8, 'NOBITS'], [9, 'REL'],
  [11, 'DYNSYM'], [14, 'INIT_ARRAY'], [15, 'FINI_ARRAY'], [16, 'PREINIT_ARRAY'],
  [17, 'GROUP'], [18, 'SYMTAB_SHNDX']]);
const machineNames = new Map([[62, 'x86-64'], [183, 'AArch64']]);
const elfTypes = new Map([[1, 'relocatable object'], [2, 'executable'], [3, 'shared object'], [4, 'core file']]);

export function describeElf(bytes, showMeaning = false, disassemble) {
  if (bytes.length < 64 || bytes[0] !== 0x7f || bytes[1] !== 0x45 || bytes[2] !== 0x4c || bytes[3] !== 0x46) {
    return 'Not a complete ELF64 object.';
  }
  if (bytes[4] !== 2 || bytes[5] !== 1) return 'ELF format is not 64-bit little-endian.';
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  const u16 = offset => view.getUint16(offset, true);
  const u32 = offset => view.getUint32(offset, true);
  const u64 = offset => view.getBigUint64(offset, true);
  const lines = ['ELF64 · little-endian · rows show file offsets and the bytes for each field.', ''];
  const field = (offset, size, name, value, note = '') => {
    const raw = hex(bytes.subarray(offset, offset + size)).padEnd(23);
    const meaning = showMeaning ? `: ${value}${note ? ` (${note})` : ''}` : '';
    lines.push(`${numberHex(offset).padEnd(8)} ${raw} ${name}${meaning}`);
  };
  lines.push('ELF header');
  field(0, 4, 'Magic', 'ELF');
  field(4, 1, 'Class', '2', 'ELF64');
  field(5, 1, 'Data encoding', '1', 'little-endian');
  field(6, 1, 'Header version', String(bytes[6]));
  field(7, 1, 'OS ABI', String(bytes[7]), bytes[7] === 0 ? 'System V' : 'ABI-specific');
  field(8, 1, 'ABI version', String(bytes[8]));
  field(16, 2, 'Object type', numberHex(u16(16)), elfTypes.get(u16(16)) ?? 'unknown');
  field(18, 2, 'Machine', numberHex(u16(18)), machineNames.get(u16(18)) ?? 'unknown');
  field(20, 4, 'ELF version', numberHex(u32(20)));
  field(24, 8, 'Entry point', numberHex(u64(24)), 'address to start execution');
  field(32, 8, 'Program header offset', numberHex(u64(32)));
  field(40, 8, 'Section header offset', numberHex(u64(40)));
  field(48, 4, 'Flags', numberHex(u32(48)));
  field(52, 2, 'ELF header size', `${u16(52)} bytes`);
  field(54, 2, 'Program header size', `${u16(54)} bytes`);
  field(56, 2, 'Program header count', String(u16(56)));
  field(58, 2, 'Section header size', `${u16(58)} bytes`);
  field(60, 2, 'Section count', String(u16(60)));
  field(62, 2, 'Section-name table index', String(u16(62)));

  const sectionOffset = Number(u64(40));
  const sectionSize = u16(58);
  const sectionCount = u16(60);
  const stringIndex = u16(62);
  if (!sectionOffset || !sectionSize || sectionCount * sectionSize + sectionOffset > bytes.length) return lines.join('\n');
  const sections = Array.from({ length: sectionCount }, (_, index) => {
    const offset = sectionOffset + index * sectionSize;
    return { offset, nameOffset: u32(offset), type: u32(offset + 4), fileOffset: u64(offset + 24),
      size: u64(offset + 32), link: u32(offset + 40), info: u32(offset + 44),
      flags: u64(offset + 8), address: u64(offset + 16), alignment: u64(offset + 48),
      entrySize: u64(offset + 56) };
  });
  const strings = sections[stringIndex];
  const nameAt = offset => {
    if (!strings || strings.fileOffset + strings.size > BigInt(bytes.length)) return '<unknown>';
    const start = Number(strings.fileOffset) + offset;
    if (start >= bytes.length) return '<unknown>';
    let end = start;
    while (end < bytes.length && bytes[end] !== 0) end++;
    return new TextDecoder().decode(bytes.subarray(start, end)) || '<unnamed>';
  };
  lines.push('', 'Section table · each section describes a range or metadata inside this object.');
  for (let index = 0; index < sections.length; index++) {
    const section = sections[index];
    const description = showMeaning ? `: ${nameAt(section.nameOffset)} · ${typeNames.get(section.type) ?? `type ${section.type}`}` : '';
    lines.push('', `Section ${index}${description}`);
    const offset = section.offset;
    field(offset, 4, 'Name string offset', numberHex(section.nameOffset));
    field(offset + 4, 4, 'Section type', numberHex(section.type), typeNames.get(section.type) ?? 'processor or OS-specific');
    field(offset + 8, 8, 'Flags', numberHex(section.flags));
    field(offset + 16, 8, 'Address', numberHex(section.address));
    field(offset + 24, 8, 'File offset', numberHex(section.fileOffset));
    field(offset + 32, 8, 'Size', `${section.size} bytes`);
    field(offset + 40, 4, 'Link', String(section.link), 'section index');
    field(offset + 44, 4, 'Info', String(section.info));
    field(offset + 48, 8, 'Alignment', String(section.alignment));
    field(offset + 56, 8, 'Entry size', `${section.entrySize} bytes`);
    if (section.type === 1 && (section.flags & 4n) !== 0n && section.fileOffset + section.size <= BigInt(bytes.length)) {
      const start = Number(section.fileOffset);
      const code = bytes.subarray(start, start + Number(section.size));
      lines.push(`  Contents · file offset ${numberHex(start)} · ${code.length} bytes`);
      if (disassemble) {
        const instructions = disassemble(code, Number(section.address));
        const byteFields = instructions.map(instruction => Array.from(instruction.bytes, byte => byte.toString(16).padStart(2, '0')).join(' '));
        const width = Math.max(23, ...byteFields.map(field => field.length));
        for (const [index, instruction] of instructions.entries()) {
          const instructionOffset = start + instruction.offset;
          lines.push(`${numberHex(instructionOffset).padEnd(8)} ${byteFields[index].padEnd(width)} ${instruction.text}`);
        }
      } else {
        for (let offset = 0; offset < code.length; offset += 16) {
          lines.push(`${numberHex(start + offset).padEnd(8)} ${hex(code.subarray(offset, offset + 16)).replace(/(..)/g, '$1 ').trimEnd()}`);
        }
      }
    }
  }
  return lines.join('\n');
}
