#include <stdio.h>
#include <unistd.h>

int main(int argc, char *argv[]) {
    int ret = getuid();
    printf("[Cage | getuid] getuid ret = %d\n", ret);
    
    printf("[Cage | getuid2] getuid ret = %d\n", getuid());
    
    return 0;
}
