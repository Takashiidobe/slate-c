// EXIT: 42
typedef void Nothing;
int count;
Nothing add(int value) { count += value; }
Nothing forward(void) { return add(42); }
int main(void) {
    forward();
    (Nothing)count;
    return count;
}
