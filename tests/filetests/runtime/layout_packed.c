// EXIT: 42
struct __attribute__((packed)) Packed { char tag; long value; };
typedef struct Packed Packed;
Packed global[2];
int main(void) {
    Packed local[2];
    if (sizeof(Packed) != 9 || _Alignof(Packed) != 1) return 1;
    if ((unsigned long)&local[1] - (unsigned long)&local[0] != 9) return 2;
    if ((unsigned long)&local[0].value - (unsigned long)&local[0] != 1) return 3;
    local[1].value = 42;
    global[1] = local[1];
    return global[1].value;
}
