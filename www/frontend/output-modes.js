CodeMirror.defineMode('slate-clif', () => ({
  startState: () => ({ opcode: true }),
  token(stream, state) {
    if (stream.sol()) state.opcode = true;
    if (stream.eatSpace()) return null;
    if (stream.match(/;.*/)) return 'comment';
    if (stream.match(/%[A-Za-z0-9_.$-]+/)) return 'def';
    if (stream.match(/block[0-9]+\b/)) return 'tag';
    if (stream.match(/(?:v|ss|gv|fn|jt)[0-9]+\b/)) return 'variable-2';
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
    if (stream.match(/[%]?(?:r(?:[0-9]+[bwd]?|ax|bx|cx|dx|si|di|bp|sp|ip)|e(?:ax|bx|cx|dx|si|di|bp|sp|ip)|[abcd][lh]|[abcd]x|[sd]il|[bs]pl|[xyz]mm[0-9]+|[xw][0-9]+|[bhsdqv][0-9]+(?:\.[0-9]+[bhsd])?|[xw]zr|sp|wsp|pc|lr|fp|nzcv|[cdefgs]s)\b/i)) return 'variable-2';
    if (stream.match(/[$#]?[+-]?(?:0x[0-9a-f]+|[0-9]+(?:\.[0-9]+)?)/i)) return 'number';
    if (stream.match(/(?:byte|word|dword|qword|xmmword|ymmword|zmmword|ptr)\b/i)) return 'type';
    if (stream.match(/[A-Za-z_.][A-Za-z0-9_.$]*/)) {
      if (state.mnemonic) { state.mnemonic = false; return 'builtin'; }
      return 'variable';
    }
    return /[+\-*$#!]/.test(stream.next()) ? 'operator' : null;
  },
}));
