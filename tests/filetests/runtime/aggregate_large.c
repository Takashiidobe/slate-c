// EXIT: 42
struct Large { long values[16]; };
int main(void) {
    struct Large original;
    for (int i = 0; i < 16; i++) original.values[i] = i;
    struct Large copy = original;
    original.values[0] = 100;
    copy = copy;
    copy.values[15] = 42;
    original = copy;
    return original.values[0] + original.values[15];
}
