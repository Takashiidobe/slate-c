// EXIT: 42
struct __attribute__((packed)) Packed { char tag; long value; int tail; };
struct __attribute__((aligned(64))) Wide { int x; int y; };
int main(void) {
    struct Packed packed = { 1, 17 };
    struct Wide wide = { 25 };
    if (packed.tag != 1 || packed.tail || wide.y) return 1;
    if ((unsigned long)&wide % 64) return 2;
    return packed.value + wide.x;
}
