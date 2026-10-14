// MODE: compile
// CLIF: check: ; copy
// CLIF: check: $(destination=$var) = load.i64
// CLIF: check: $(source=$var) = load.i64
// CLIF: check: $(snapshot=$var) = stack_addr.i64
// CLIF: nextln: $(data=$var) = load.i64 $source
// CLIF: nextln: store $data, $snapshot
// CLIF: nextln: $(saved=$var) = load.i64 $snapshot
// CLIF: nextln: store $saved, $destination
struct Pair { int x; int y; };
void copy(struct Pair *destination, struct Pair *source) {
    *destination = *source;
}
