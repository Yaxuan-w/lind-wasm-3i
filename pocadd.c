#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>

int pass_fptr_to_wt() {
    int fd = open("testfile.txt", O_CREAT | O_WRONLY, 0644);
    if (fd == -1) {
        return -1;
    }
    return fd;
}

// Required to be loaded in wasmtime
int main() {
	return 0;
}
