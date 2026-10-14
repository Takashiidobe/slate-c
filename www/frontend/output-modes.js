CodeMirror.defineMode('slate-clif', () => ({
  startState: () => ({ opcode: true }),
  token(stream, state) {
    if (stream.sol()) state.opcode = true;
    if (stream.eatSpace()) return null;
    if (stream.match(/;.*/)) return 'comment';
    if (stream.match(/%[A-Za-z0-9_.$-]+/)) return 'def';
    if (stream.match(/block[0-9]+\b/)) return 'tag';
    if (stream.match(/v[0-9]+\b/)) return 'clif-value';
    if (stream.match(/(?:ss|gv|fn|jt)[0-9]+\b/)) return 'clif-entity';
    if (stream.match(/(?:i|b|f)[0-9]+(?:x[0-9]+)?\b/)) return 'type';
    if (stream.match(/[+-]?(?:0x[0-9A-Fa-f.]+|[0-9]+(?:\.[0-9]*)?(?:[eE][+-]?[0-9]+)?)/)) return 'number';
    if (stream.match(/"(?:[^"\\]|\\.)*"/)) return 'string';
    if (stream.match(/(?:function|test|target|set)\b/)) { state.opcode = false; return 'keyword'; }
    if (stream.match(/(?:system_v|fast|cold|tail|windows_fastcall|apple_aarch64|winch)\b/)) return 'type';
    if (stream.match(/[A-Za-z_][A-Za-z0-9_$]*/)) {
      if (state.opcode) { state.opcode = false; return 'builtin'; }
      return 'variable';
    }
    const char = stream.next();
    if (char === '=') { state.opcode = true; return 'operator'; }
    return /[+\-*/<>!&|]/.test(char) ? 'operator' : null;
  },
}));

CodeMirror.defineMode('slate-assembly', () => ({
  startState: () => ({ mnemonic: true }),
  token(stream, state) {
    if (stream.sol()) {
      state.mnemonic = true;
      if (stream.match(/[^\s:]+:/)) { state.mnemonic = false; return 'def'; }
    }
    if (stream.eatSpace()) return null;
    if (stream.match(/;.*/)) return 'comment';
    if (stream.match(/"(?:[^"\\]|\\.)*"/)) return 'string';
    if (stream.match(/%?(?:r(?:[0-9]+[bwd]?|ax|bx|cx|dx|si|di|bp|sp|ip|flags)|e(?:ax|bx|cx|dx|si|di|bp|sp|ip)|[abcd][lh]|[abcd]x|[sd]il|[bs]pl|[xyz]mm[0-9]+|[xw][0-9]+|[bhsdqv][0-9]+(?:\.[0-9]+[bhsd])?|[xw]zr|sp|wsp|pc|lr|fp|nzcv|[cdefgs]s)\b/i)) return 'asm-register';
    if (stream.match(/[$#]?[+-]?(?:0x[0-9a-f]+|[0-9]+(?:\.[0-9]+)?)/i)) return 'number';
    if (stream.match(/(?:byte|word|dword|qword|xmmword|ymmword|zmmword|ptr)\b/i)) return 'type';
    if (stream.match(/[A-Za-z_.][A-Za-z0-9_.$]*/)) {
      if (state.mnemonic) { state.mnemonic = false; return 'builtin'; }
      return 'variable';
    }
    return /[+\-*$#!]/.test(stream.next()) ? 'operator' : null;
  },
}));

CodeMirror.defineMode('slate-sexpr', () => ({
  startState: () => ({ head: false }),
  token(stream, state) {
    if (stream.eatSpace()) return null;
    if (stream.match(/"(?:[^"\\]|\\.)*"/)) return 'string';
    if (stream.match(/[+-]?(?:0x[0-9a-f]+|[0-9]+(?:\.[0-9]+)?)/i)) return 'number';
    if (stream.eat('(')) { state.head = true; return 'bracket'; }
    if (stream.eat(')')) return 'bracket';
    if (stream.match(/[^\s()]+/)) {
      if (state.head) { state.head = false; return 'builtin'; }
      return 'variable';
    }
    stream.next();
    return null;
  },
}));

CodeMirror.defineMode('slate-tokens', () => ({
  startState: () => ({ headerLines: 2, field: 0, spellingStyle: null, keywordKind: false }),
  token(stream, state) {
    if (stream.sol()) {
      state.field = 0;
      state.spellingStyle = null;
      state.keywordKind = false;
      if (state.headerLines) {
        state.headerLines--;
        stream.skipToEnd();
        return 'comment';
      }
    }
    if (state.field === 0 && stream.eat('\t')) { state.field = 1; return null; }
    if (state.field === 1) {
      stream.skipToEnd();
      return state.spellingStyle;
    }
    if (stream.eatSpace()) return null;
    if (stream.match(/"(?:[^"\\]|\\.)*"/)) return 'string';
    if (stream.match(/\d+/)) return 'number';
    if (stream.match(/[A-Z][A-Za-z0-9_]*/)) {
      const name = stream.current();
      if (name === 'Keyword') { state.keywordKind = true; state.spellingStyle = 'keyword'; return 'keyword'; }
      if (name === 'Ident') { state.spellingStyle = 'variable'; return 'variable'; }
      if (/^(?:IntLit|FloatLit)$/.test(name)) { state.spellingStyle = 'number'; return 'type'; }
      if (/^(?:CharLit|Utf\d*CharLit|WideCharLit|StringLit|Utf\d*StringLit|WideStringLit)$/.test(name)) { state.spellingStyle = 'string'; return 'type'; }
      if (name === 'Comment') { state.spellingStyle = 'comment'; return 'comment'; }
      if (name === 'Newline') { state.spellingStyle = 'comment'; return 'comment'; }
      return state.keywordKind ? 'keyword' : 'type';
    }
    if (stream.match(/[a-z_][A-Za-z0-9_]*/)) return 'property';
    if (stream.match(/[()[\]{}]/)) return 'bracket';
    if (stream.match(/[,:]/)) return 'punctuation';
    stream.next();
    return 'operator';
  },
}));

CodeMirror.defineMode('slate-elf', () => ({
  startState: () => ({ byteStart: 9, dataEnd: 32, lineOffset: -1 }),
  token(stream, state) {
    if (stream.sol()) {
      state.byteStart = 9;
      state.dataEnd = 32;
      state.lineOffset = -1;
      if (stream.match(/^ELF64\b.*/)) return 'comment';
      if (stream.match(/^ELF header$/)) return 'keyword';
      if (stream.match(/^Section table.*/)) return 'comment';
      if (stream.match(/^Section \d+.*/)) return 'def';
      if (stream.match(/^(0x[0-9a-f]+)/i)) {
        const offset = stream.current();
        state.lineOffset = Number.parseInt(offset.slice(2), 16);
        state.byteStart = Math.max(8, offset.length) + 1;
        state.dataEnd = state.byteStart + 23;
        return 'meta';
      }
    }
    const column = stream.column();
    if (column >= state.byteStart && column < state.dataEnd) {
      if (stream.match(/[0-9a-f]{2}/i)) {
        const byte = stream.current().toLowerCase();
        if (state.lineOffset === 0) return 'elf-magic';
        return byte === '00' ? 'elf-zero' : 'elf-byte';
      }
    }
    if (stream.eatSpace()) return null;
    if (stream.eat(':')) return 'punctuation';
    if (stream.match(/[A-Za-z][A-Za-z0-9 _-]*/)) return 'property';
    if (stream.match(/0x[0-9a-f]+/i)) return 'number';
    stream.next();
    return null;
  },
}));
