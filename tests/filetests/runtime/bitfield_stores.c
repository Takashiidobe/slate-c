// EXIT: 0
struct Fields { unsigned int low : 3; signed int signed_value : 4; unsigned int high : 5; };
struct Offset { unsigned int prefix; signed int value : 5; };

int main(void) {
    unsigned int storage = 0x7fd;
    struct Fields *fields = (struct Fields *)&storage;
    fields->low = 2;
    if (fields->low != 2 || fields->signed_value != -1 || fields->high != 15) return 1;
    if ((fields->low = 3) != 3 || fields->signed_value != -1 || fields->high != 15) return 2;
    fields->low += 3;
    if (fields->low != 6) return 3;
    if (fields->low++ != 6 || fields->low != 7) return 4;
    if (++fields->low != 0 || fields->signed_value != -1 || fields->high != 15) return 5;

    unsigned int units[2] = { 41, 0 };
    struct Offset *offset = (struct Offset *)units;
    offset->value = -7;
    if (offset->prefix != 41 || offset->value != -7) return 6;
    offset->value += 2;
    if (offset->prefix != 41 || offset->value != -5) return 7;
    return 0;
}
