// EXIT: 0
struct Separated {
    unsigned low : 3;
    unsigned : 0;
    signed high : 5;
};
int main(void) {
    unsigned storage[3] = { 0, 0, 0xa5a5a5a5 };
    struct Separated *p = (struct Separated *)storage;
    if (sizeof(struct Separated) != 8) return 1;
    p->low = 5;
    p->high = -7;
    if (storage[0] != 5 || storage[1] != 25) return 2;
    if (p->low != 5 || p->high != -7) return 3;
    if (++p->high != -6 || p->low != 5) return 4;
    return storage[2] != 0xa5a5a5a5;
}
