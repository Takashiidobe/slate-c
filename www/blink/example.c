#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    char *message = malloc(64);
    if (!message) return 1;
    strcpy(message, "Hello from musl in Blink");
    printf("%s: %d\n", message, 42);
    free(message);
    FILE *file = fopen("/tmp/guest.txt", "w+");
    if (!file) return 2;
    fputs("virtual file", file);
    rewind(file);
    char buffer[32];
    if (!fgets(buffer, sizeof buffer, file)) return 3;
    printf("Read: %s\n", buffer);
    fclose(file);
    return 42;
}
