#include <stdio.h>
#include <register_handler.h>

int main() {
    int ret = register_handler(1, 1, 1, 1);
    printf("ret: %d", ret);
    return 0;
}