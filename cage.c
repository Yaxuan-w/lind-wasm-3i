#include <stdio.h>
#include <unistd.h>

int main() {
    sleep(20);
    int ret = dup2(1, 2);
    printf("mkdir in cage ret: %d", ret);
    return 0;
}
