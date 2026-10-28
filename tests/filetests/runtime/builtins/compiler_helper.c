// EXIT: 0
extern int __popcountdi2(unsigned long long);
int main(void) {
    if (__popcountdi2(0) != 0) return 1;
    if (__popcountdi2(0xffffffffffffffffULL) != 64) return 2;
    return __popcountdi2(0xf000000000000001ULL) != 5;
}
