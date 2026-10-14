// MODE: compile
// CLIF: check: ; diff_char
// CLIF: check: $(diff=$var) = isub $var, $var
// CLIF: nextln: return $diff
// CLIF: check: ; diff_int
// CLIF: check: $(diff=$var) = isub $var, $var
// CLIF: check: $(res=$var) = sdiv $diff, $var
// CLIF: nextln: return $res
// CLIF: check: ; diff_nested
// CLIF: check: $(diff=$var) = isub $var, $var
// CLIF: check: $(res=$var) = sdiv $diff, $var
// CLIF: nextln: return $res
long diff_char(char *p1, char *p2) {
    return p1 - p2;
}

long diff_int(int *p1, int *p2) {
    return p1 - p2;
}

long diff_nested(int (*p1)[4], int (*p2)[4]) {
    return p1 - p2;
}
