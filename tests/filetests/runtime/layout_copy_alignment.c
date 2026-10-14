// EXIT: 42
struct __attribute__((aligned(64))) Wide { int value; };
int main(void) {
    struct Wide values[2];
    struct Wide copy;
    values[0].value = 17;
    values[1].value = 25;
    if ((unsigned long)&values[0] % 64 || (unsigned long)&values[1] % 64) return 1;
    if ((unsigned long)&values[1] - (unsigned long)&values[0] != 64) return 2;
    int choose_first = 1;
    copy = choose_first ? values[0] : values[1];
    if (copy.value != 17) return 3;
    copy = !choose_first ? values[0] : values[1];
    if (copy.value != 25) return 4;
    if ((unsigned long)&copy % 64) return 5;
    return values[0].value + copy.value;
}
