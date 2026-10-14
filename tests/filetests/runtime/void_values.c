// EXIT: 42
int count;
void add(int value) { count += value; }
void forward(void) { return add(10); }
int main(void) {
    (void)(count = 2);
    forward();
    count == 12 ? add(20) : add(100);
    count == 0 ? add(100) : add(10);
    (add(0), (void)count);
    return count;
}
