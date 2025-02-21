#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>

typedef int (*func_ptr_t)(uintptr_t, uintptr_t, uintptr_t, uintptr_t, uintptr_t, uintptr_t);
int open_grate(uintptr_t, uintptr_t, uintptr_t, uintptr_t, uintptr_t, uintptr_t);
int add(uintptr_t, uintptr_t, uintptr_t, uintptr_t, uintptr_t, uintptr_t);

func_ptr_t func_array[2] = {open_grate, add};

int pass_fptr_to_wt(int index, uintptr_t arg1, uintptr_t arg2, uintptr_t arg3, uintptr_t arg4, uintptr_t arg5, uintptr_t arg6) {
    if (index < 0 || index >= 2) {
        fprintf(stderr, "Invalid index: %d\n", index);
        return -1; 
    }
    
    return func_array[index](arg1, arg2, arg3, arg4, arg5, arg6);
}

int open_grate(uintptr_t arg1, uintptr_t arg2, uintptr_t arg3, uintptr_t arg4, uintptr_t arg5, uintptr_t arg6) {
    int fd = open("testfile.txt", O_CREAT | O_WRONLY, 0644);
    if (fd == -1) {
        perror("open_grate failed");
        return -1;
    }
    return fd;
}

int add(uintptr_t a, uintptr_t b, uintptr_t arg3, uintptr_t arg4, uintptr_t arg5, uintptr_t arg6) {
    return a + b;
}

// Required to be loaded in wasmtime
int main() {
    return 0;
}
