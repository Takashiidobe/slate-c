// EXIT: 0
struct Boundary {
    unsigned first : 31;
    signed second : 3;
    unsigned third : 29;
    unsigned long long full : 64;
};
int main(void) {
    unsigned long long storage[3] = { 0, 0, 0xa5a5a5a5a5a5a5a5ULL };
    struct Boundary *p = (struct Boundary *)storage;
    if (sizeof(struct Boundary) != 16) return 1;
    p->first = 0x76543210;
    p->second = -3;
    p->third = 0x1234567;
    p->full = 0xfedcba9876543210ULL;
    if (p->first != 0x76543210 || p->second != -3 || p->third != 0x1234567) return 2;
    if (storage[0] != 0x091a2b3d76543210ULL || storage[1] != 0xfedcba9876543210ULL) return 3;
    if (p->second++ != -3 || p->second != -2 || p->third != 0x1234567) return 4;
    if (p->full-- != 0xfedcba9876543210ULL || p->full != 0xfedcba987654320fULL) return 5;
    return storage[2] != 0xa5a5a5a5a5a5a5a5ULL;
}
