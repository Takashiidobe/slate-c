// MODE: compile
// CLIF: check: ; read_middle
// CLIF: check: $(first=$var) = load.i8 $(address=$var)
// CLIF: check: $(second=$var) = load.i8 $address+1
// CLIF: not: load.i8
// CLIF: check: sshr
// CLIF: check: return
// CLIF: check: ; write_middle
// CLIF: check: $(old=$var) = load.i8 $(destination=$var)
// CLIF: check: store $var, $destination
// CLIF: check: store $var, $destination+1
// CLIF: not: store
// CLIF: check: return
struct __attribute__((packed)) Packed { unsigned low : 3; signed middle : 13; unsigned high : 5; };
long long read_middle(struct Packed *p) { return p->middle; }
void write_middle(struct Packed *p, int value) { p->middle = value; }
