// EXIT: 42
int main(void) {
    int full[3] = { 1, 2, 3 };
    int partial[4] = { 17, 25 };
    int inferred[] = { 4, 5 };
    int matrix[2][3] = { { 1, 2 }, { 3 } };
    int elided[2][3] = { 1, 2, 3, 4 };
    if (full[0] != 1 || full[2] != 3) return 1;
    if (partial[2] || partial[3]) return 2;
    if (sizeof(inferred) != 8 || inferred[1] != 5) return 3;
    if (matrix[0][2] || matrix[1][1] || matrix[1][2]) return 4;
    if (elided[1][0] != 4 || elided[1][1] || elided[1][2]) return 5;
    return partial[0] + partial[1];
}
