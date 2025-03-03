#include <stdio.h>
#include <unistd.h>

int main(int argc, char *argv[]) {
    int ret = getuid();
    printf("[Cage | getuid] getuid ret = %d\n", ret);
    return 0;
}
