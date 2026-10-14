// EXIT: 7
// STDOUT: check: hello stdout
// STDERR: check: hello stderr
extern long write(int fd, const void *buffer, unsigned long length);
int main(void) {
    write(1, "hello stdout\n", 13);
    write(2, "hello stderr\n", 13);
    return 7;
}
