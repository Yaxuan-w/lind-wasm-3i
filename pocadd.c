#include <stdio.h>

int add_def(int a, int b) { return a + b; }

extern void test_func(int (*add)(int, int));

int main() {
    test_func(add_def);
}
