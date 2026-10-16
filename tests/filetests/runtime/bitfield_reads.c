// EXIT: 0
struct Fields { unsigned int low : 3; signed int signed_value : 4; unsigned int high : 5; };
struct Offset { int prefix; signed int value : 5; };

int main(void) {
    unsigned int storage = 0x7fd;
    struct Fields *fields = (struct Fields *)&storage;
    if (fields->low != 5 || fields->signed_value != -1 || fields->high != 15) return 1;

    unsigned int units[2] = { 0, 0x1f };
    struct Offset *offset = (struct Offset *)units;
    if (offset->value != -1) return 2;
    return 0;
}
