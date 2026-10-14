// EXIT: 42
struct Scalars { signed char small; unsigned short short_value; float real; double wide; int *pointer; _Bool flag; };
int main(void) {
    volatile int unused;
    int value = 25;
    struct Scalars scalars = { -1, 17, 3.5f, 4.5, &value, 1 };
    if (scalars.small != -1 || scalars.real != 3.5f || scalars.wide != 4.5) return 1;
    if (!scalars.flag || scalars.pointer != &value) return 2;
    return scalars.short_value + *scalars.pointer;
}
