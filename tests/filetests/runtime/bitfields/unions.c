// EXIT: 0
union Bits {
    unsigned raw;
    signed all : 32;
    struct { unsigned low : 3; signed middle : 5; unsigned high : 24; } parts;
};
union Full { unsigned long long raw; unsigned long long field : 64; };
union __attribute__((packed)) Small { unsigned value : 17; signed signed_value : 17; };
int main(void) {
    union Bits bits;
    bits.raw = 0x123456ed;
    if (bits.parts.low != 5 || bits.parts.middle != -3 || bits.parts.high != 0x123456) return 1;
    bits.parts.middle = -7;
    if (bits.raw != 0x123456cd) return 2;
    bits.all = -1;
    if (bits.raw != 0xffffffff || bits.all != -1) return 3;
    union Full full;
    full.raw = 0xffffffffffffffffULL;
    if (++full.field != 0 || full.raw != 0) return 4;
    unsigned char storage[5] = { 0x5a, 0, 0, 0, 0xa5 };
    union Small *small = (union Small *)(storage + 1);
    if (sizeof(union Small) != 3) return 5;
    small->value = 0x1fffd;
    if (small->signed_value != -3) return 6;
    small->signed_value += 2;
    if (small->value != 0x1ffff) return 7;
    return storage[0] != 0x5a || storage[4] != 0xa5;
}
