// MODE: compile
// CLIF: check: explicit_slot 1$(=\n)
// CLIF: check: explicit_slot 4, align = 4
// CLIF: check: explicit_slot 8, align = 8
// CLIF: check: explicit_slot 17, align = 16
// CLIF: check: $(slot=ss[0-9]+) = explicit_slot 64, align = 16
// CLIF: check: $(base=$var) = stack_addr.i64 $slot
// CLIF: nextln: $(padding=$var) = iconst.i64 63
// CLIF: nextln: $(offset=$var) = iadd $base, $padding
// CLIF: nextln: $(mask=$var) = iconst.i64 -64
// CLIF: nextln: $var = band $offset, $mask
void storage(void) {
    char byte;
    int integer;
    double floating;
    char array[17];
    _Alignas(64) char aligned;
}
