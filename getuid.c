#include <stdio.h>
#include <unistd.h>

int main(int argc, char *argv[]) {
    int ret = getuid();
    printf("[cage | getuid] getuid ret = %d", ret);
    return 0;
}
