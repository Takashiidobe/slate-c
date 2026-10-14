// EXIT: 42
struct Pair { int x; int y; };
int main(void) {
    struct Pair left;
    struct Pair right;
    left.x = 17;
    left.y = 25;
    right.x = 100;
    right.y = 200;
    int yes = 1;
    struct Pair result = yes ? left : right;
    result = !yes ? right : result;
    return result.x + result.y;
}
