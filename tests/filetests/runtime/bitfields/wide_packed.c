// EXIT: 0
struct __attribute__((packed)) Wide {
    unsigned long long low : 7;
    unsigned long long full : 64;
    unsigned tail : 1;
};
int main(void) {
    unsigned char storage[11] = { 0x5a, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xa5 };
    struct Wide *p = (struct Wide *)(storage + 1);
    if (sizeof(struct Wide) != 9) return 1;
    p->low = 85;
    p->tail = 1;
    p->full = 0xfedcba9876543210ULL;
    if (p->low != 85 || p->tail != 1 || p->full != 0xfedcba9876543210ULL) return 2;
    if (storage[0] != 0x5a || storage[10] != 0xa5) return 3;
    if (storage[1] != 0x55 || storage[2] != 8 || storage[9] != 0xff) return 4;
    if (p->full++ != 0xfedcba9876543210ULL || p->full != 0xfedcba9876543211ULL) return 5;
    p->full = 0xffffffffffffffffULL;
    if (++p->full != 0 || p->low != 85 || p->tail != 1) return 6;
    return 0;
}
