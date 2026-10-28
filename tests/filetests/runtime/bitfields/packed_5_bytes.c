// EXIT: 0
struct __attribute__((packed)) Packed {
    unsigned low : 3;
    signed long long value : 33;
};
int main(void) {
    unsigned char storage[7];
    for (int i = 0; i < 7; ++i) storage[i] = 0xa5;
    struct Packed *p = (struct Packed *)(storage + 1);
    if (sizeof(struct Packed) != 5) return 1;
    p->low = 5;
    p->value = -3;
    if (p->value != -3 || p->low != 5) return 2;
    if (p->value++ != -3 || p->value != -2) return 3;
    if (storage[0] != 0xa5 || storage[6] != 0xa5) return 4;
    if ((storage[5] & 0xf0) != 0xa0 || p->low != 5) return 5;
    p->value = (1LL << 32) - 1;
    if (++p->value != -(1LL << 32)) return 6;
    return 0;
}
