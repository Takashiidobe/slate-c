// MODE: error
// DIAGNOSTIC: lowering
// CODE: slate_c::lowering
// ERROR: check: atomic and volatile access is not supported
// ERROR: check: lowering failed here
struct __attribute__((packed)) Bits { volatile unsigned value : 17; };
int access(struct Bits *p) { p->value = 3; return 0; }
