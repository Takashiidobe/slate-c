// TARGET: x86_64-unknown-linux-gnu
// TARGET: aarch64-unknown-linux-gnu
// FUNCTIONS: 2
// CLIF: check: ; read_full
// CLIF: check: load.i8
// CLIF: check: load.i8 $var+8
// CLIF: check: return
// CLIF: check: ; write_full
// CLIF: check: store $var, $var+8
// CLIF: check: return
struct __attribute__((packed)) Wide {
    unsigned low : 7;
    unsigned long long full : 64;
    unsigned tail : 1;
};
unsigned long long read_full(struct Wide *p) { return p->full; }
void write_full(struct Wide *p, unsigned long long value) { p->full = value; }
