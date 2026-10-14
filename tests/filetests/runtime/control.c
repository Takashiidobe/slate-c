// EXIT: 14
int global = 2;
int add(int x, int y) { return x + y; }
int main(void) {
    int sum = 0;
    for (int i = 0; i < 5; i++) {
        if (i == 3) continue;
        sum = add(sum, i);
    }
    int values[2];
    values[0] = sum;
    values[1] = global;
    return values[0] * values[1];
}
