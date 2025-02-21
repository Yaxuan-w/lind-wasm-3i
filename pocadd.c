#include <fcntl.h>
#include <unistd.h>
#include <stdio.h>

int pass_fptr_to_wt(int a, int b) {
    return a + b;
}

// Required to be loaded in wasmtime
int main() {
	return 0;
}
