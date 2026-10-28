// EXIT: 0
struct __attribute__((packed)) Packed {
    unsigned low : 3;
    signed middle : 13;
    unsigned high : 5;
};
int main(void) {
    unsigned char storage[5] = { 0x5a, 0, 0, 0, 0xa5 };
    struct Packed *p = (struct Packed *)(storage + 1);
    if (sizeof(struct Packed) != 3) return 1;
    p->low = 5;
    p->middle = -123;
    p->high = 27;
    if (p->low != 5 || p->middle != -123 || p->high != 27) return 2;
    if (storage[0] != 0x5a || storage[4] != 0xa5) return 3;
    if (storage[1] != 0x2d || storage[2] != 0xfc || storage[3] != 27) return 4;
    if ((p->low = 15) != 7 || p->low != 7) return 5;
    if (p->middle++ != -123 || p->middle != -122) return 6;
    if ((p->high += 6) != 1 || p->high != 1) return 7;
    if (p->low != 7 || storage[0] != 0x5a || storage[4] != 0xa5) return 8;
    return 0;
}
