// EXIT: 42
struct Object { int tag; int values[64]; };
int main(void) {
    int values[128] = { 17, 25 };
    struct Object object = { 42 };
    for (int i = 2; i < 128; i++) if (values[i]) return 1;
    for (int i = 0; i < 64; i++) if (object.values[i]) return 2;
    return values[0] + values[1];
}
