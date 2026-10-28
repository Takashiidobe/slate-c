// MODE: error
// DIAGNOSTIC: lowering
// CODE: slate_c::lowering
// LABEL: switch
// LABEL-MIN: 7
// ERROR: check: switch statements are not supported
// ERROR: check: input.c
// ERROR: check: label at line
/* λ */
int main(void) { if (1) { switch (1) { default: return 0; } } }
