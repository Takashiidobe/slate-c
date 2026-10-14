// EXIT: 42
typedef int WideInt __attribute__((aligned(64)));
int check(int value, double floating) {
    if ((unsigned long)&value % _Alignof(int)) return 1;
    if ((unsigned long)&floating % _Alignof(double)) return 2;
    WideInt *literal = &(WideInt){42};
    if ((unsigned long)literal % 64) return 3;
    return *literal;
}
int main(void) { return check(17, 25.0); }
