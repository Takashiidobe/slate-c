// MODE: compile
// CLIF: check: ; initialize
// CLIF: check: $(address=$var) = stack_addr.i64
// CLIF: nextln: $(zero=$var) = iconst.i64 0
// CLIF: nextln: store $zero, $address
// CLIF: check: $(value=$var) = iconst.i32 17
// CLIF: nextln: store $value, $var
// CLIF: check: load.i32
struct Pair { int x; int y; };
int initialize(void) {
    struct Pair pair = { 17 };
    return pair.y;
}
