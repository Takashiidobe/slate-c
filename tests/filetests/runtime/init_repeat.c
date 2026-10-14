// EXIT: 42
struct Pair { int x; int y; };
int main(void) {
    int sum = 0;
    for (int i = 0; i < 3; i++) {
        int values[3] = { i };
        struct Pair pair = { 14 };
        if (values[1] || values[2] || pair.y) return 1;
        sum += pair.x;
        values[1] = 99;
        values[2] = 100;
        pair.y = 101;
    }
    return sum;
}
