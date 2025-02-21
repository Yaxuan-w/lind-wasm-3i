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
    int targetcageid = 2;
    int targetcallnum = 1;
    void *handlefunc = NULL;
    int handlefunccage = 1;
    int ret = registerhandler(targetcageid, targetcallnum, handlefunc, handlefunccage);
    if (ret == 1) {
        printf("[GRATE] registerhandler succeed!");
    } else {
        printf("[Error - GRATE] register handler failed");
    }
	return 0;
}
