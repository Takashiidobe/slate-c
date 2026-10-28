// EXIT: 0
// CLIF: check: $(count=$var) = popcnt $var
// CLIF: check: return $count
unsigned long long count(unsigned long long value) {
    return __builtin_elementwise_popcount(value);
}
int main(void) {
    if (count(0) != 0) return 1;
    if (count(0xffffffffffffffffULL) != 64) return 2;
    return count(0xf000000000000001ULL) != 5;
}
