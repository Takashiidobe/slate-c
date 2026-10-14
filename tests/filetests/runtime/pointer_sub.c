// EXIT: 42
struct Item {
    int a;
    char b;
    int c;
};

int main(void) {
    int ints[8];
    if (&ints[7] - &ints[2] != 5) return 1;
    if (&ints[2] - &ints[7] != -5) return 2;
    if (&ints[3] - &ints[3] != 0) return 3;

    char chars[16];
    if (&chars[10] - &chars[3] != 7) return 4;
    if (&chars[3] - &chars[10] != -7) return 5;

    long longs[6];
    if (&longs[5] - &longs[1] != 4) return 6;
    if (&longs[1] - &longs[5] != -4) return 7;

    struct Item items[6];
    if (&items[5] - &items[2] != 3) return 8;
    if (&items[2] - &items[5] != -3) return 9;

    int matrix[4][5];
    int (*row_a)[5] = &matrix[3];
    int (*row_b)[5] = &matrix[1];
    if (row_a - row_b != 2) return 10;
    if (row_b - row_a != -2) return 11;

    int cube[3][4][5];
    int (*plane_a)[4][5] = &cube[2];
    int (*plane_b)[4][5] = &cube[0];
    if (plane_a - plane_b != 2) return 12;
    if (plane_b - plane_a != -2) return 13;

    return 42;
}
