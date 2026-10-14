// EXIT: 42
int calls;
int next(int value) { calls++; return value; }
struct Pair { int x; int y; };
struct Object { struct Pair pair; int values[3]; };
int main(void) {
    int source = 10;
    struct Pair original = { 17, 25 };
    struct Object object = { original, { next(source), next(source + 1) } };
    int values[3] = { next(2), next(3) };
    if (calls != 4 || object.values[0] != 10 || object.values[1] != 11) return 1;
    if (object.values[2] || values[0] != 2 || values[1] != 3 || values[2]) return 2;
    original.x = 100;
    return object.pair.x + object.pair.y;
}
