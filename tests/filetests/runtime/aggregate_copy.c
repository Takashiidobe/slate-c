// EXIT: 42
struct Pair { int x; int y; };
struct Pair global;
int main(void) {
    struct Pair source;
    source.x = 17;
    source.y = 25;
    struct Pair copy = source;
    source.x = 99;
    global = copy;
    copy = copy;
    return global.x + copy.y;
}
