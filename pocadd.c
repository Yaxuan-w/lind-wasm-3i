#include <fcntl.h>
#include <unistd.h>
#include <stdint.h>
#include <stdio.h>

typedef int (*func_ptr_t)(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);
int open_grate(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);
int add(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);

func_ptr_t func_array[2] = {open_grate, add};

/*
*   This function is used for wasmtime to extract the 
*/
int pass_fptr_to_wt(uint64_t index, uint64_t cageid, uint64_t arg1, uint64_t arg1cage, uint64_t arg2, uint64_t arg2cage, uint64_t arg3, uint64_t arg3cage, uint64_t arg4, uint64_t arg4cage, uint64_t arg5, uint64_t arg5cage, uint64_t arg6, uint64_t arg6cage) {
    if (index < 0 || index >= 2) {
        fprintf(stderr, "Invalid index: %llu\n", index);
        return -1; 
    }
    
    return func_array[index](cageid, arg1, arg1cage, arg2, arg2cage, arg3, arg3cage, arg4, arg4cage, arg5, arg5cage, arg6, arg6cage);
}

int open_grate(uint64_t cageid, uint64_t path, uint64_t arg1cage, uint64_t arg2, uint64_t arg2cage, uint64_t arg3, uint64_t arg3cage, uint64_t arg4, uint64_t arg4cage, uint64_t arg5, uint64_t arg5cage, uint64_t arg6, uint64_t arg6cage) {
    int fd = open((const char*)path, O_CREAT | O_WRONLY, 0644);
    if (fd < 0) {
        perror("open_grate failed");
        return -1;
    }
    return fd;
}

int add(uint64_t cageid, uint64_t a, uint64_t b, uint64_t arg2, uint64_t arg2cage, uint64_t arg3, uint64_t arg3cage, uint64_t arg4, uint64_t arg4cage, uint64_t arg5, uint64_t arg5cage, uint64_t arg6, uint64_t arg6cage) {
    return a + b;
}

// Required to be loaded in wasmtime
int main() {
    return 0;
}
