// EXIT: 42
struct Pair { int x; int y; };
int main(void) {
    struct Pair a;
    struct Pair b;
    struct Pair c;
    a.x = 17;
    a.y = 25;
    b.x = 100;
    b.y = 200;
    c = (a = b, b = a, a);
    a.x = 1;
    b.x = 2;
    if (c.x != 100 || c.y != 200) return 1;
    a.x = 17;
    a.y = 25;
    c = b = a;
    b.y = 99;
    return c.x + c.y;
}
