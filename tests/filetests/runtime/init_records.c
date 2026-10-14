// EXIT: 42
struct Pair { int x; int y; };
struct Object { char tag; struct Pair pair; int values[3]; double real; int *pointer; };
typedef struct Object Object;
int main(void) {
    Object object = { 1, { 17, 25 }, { 2 } };
    struct Pair pairs[3] = { { 17 }, { 25, 1 } };
    if (object.tag != 1 || object.values[0] != 2) return 1;
    if (object.values[1] || object.values[2] || object.real != 0.0 || object.pointer) return 2;
    if (pairs[0].y || pairs[1].y != 1 || pairs[2].x || pairs[2].y) return 3;
    struct Pair copy = object.pair;
    return copy.x + copy.y;
}
