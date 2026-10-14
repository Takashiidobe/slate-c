// MODE: compile
// CLIF: check: ; add
// CLIF: check: $(sum=$var) = iadd $var, $var
// CLIF: nextln: return $sum
int add(int x, int y) { return x + y; }
