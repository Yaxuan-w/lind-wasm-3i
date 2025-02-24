#include <fcntl.h>
#include <unistd.h>
#include <stdint.h>
#include <stdio.h>

typedef int (*func_ptr_t)(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);
int open_grate(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);
int add(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);

func_ptr_t func_array[2] = {open_grate, add};

int pass_fptr_to_wt(uint64_t index, uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5, uint64_t arg6) {
    if (index < 0 || index >= 2) {
        fprintf(stderr, "Invalid index: %llu\n", index);
        return -1; 
    }
    
    return func_array[index](arg1, arg2, arg3, arg4, arg5, arg6);
}

int open_grate(uint64_t arg1, uint64_t arg2, uint64_t arg3, uint64_t arg4, uint64_t arg5, uint64_t arg6) {
    int fd = open("testfile.txt", O_CREAT | O_WRONLY, 0644);
    if (fd == -1) {
        perror("open_grate failed");
        return -1;
    }
    return fd;
}

int add(uint64_t a, uint64_t b, uint64_t arg3, uint64_t arg4, uint64_t arg5, uint64_t arg6) {
    return a + b;
}

// Required to be loaded in wasmtime
int main() {
    return 0;
}
