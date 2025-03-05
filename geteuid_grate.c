#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <register_handler.h>
#include <sys/types.h>
#include <sys/wait.h>

/*
*   Because in wasmtime, it will continue to execute until the main function ends after finding the entry point of main function. 
*   Due to the limitation of lifetime on `Store` in rust language features, it is very cumbersome to insert an interrupt mechanism 
*   during execution (I haven't found a way so far), so I put the syscall interception mechanism before wasmtime executes the main 
*   function. But this will cause a problem: Every time we call the intercepted function (`geteuid` in this case) through 3i, the 
*   context info (stored in `Store` in wasmtime) accessed by 3i is before main runs, which makes it impossible to set the uid 
*   constant in grate and then get it through 3i by using the method of `./grateeuid 10 cageeuid`. (Because the main function has 
*   not started to execute when 3i accesses it, the constant accessed is set after compilation, while setting it through command 
*   line requires executing main function and then modifying context info). So I let the user modify this constant through the 
*   clang compilation flag `-DEUID_GRATE_VAL=$val`
*/
#ifndef EUID_GRATE_VAL
#define EUID_GRATE_VAL 10
#endif

// Grate function implementation
int geteuid_grate(uint64_t cageid, uint64_t arg1, uint64_t arg1cage, uint64_t arg2, uint64_t arg2cage, uint64_t arg3, uint64_t arg3cage, uint64_t arg4, uint64_t arg4cage, uint64_t arg5, uint64_t arg5cage, uint64_t arg6, uint64_t arg6cage) {
    printf("[Grate | geteuid] current grateid: %d, geteuid: %d", getpid(), EUID_GRATE_VAL);
    return EUID_GRATE_VAL;
}

int main(int argc, char *argv[]) {
    // Should be at least two inputs (at least one grate file and one cage file)
    if (argc < 2) {
        fprintf(stderr, "Usage: %s <cage_file> <grate_file> <cage_file> [...]\n", argv[0]);
        exit(EXIT_FAILURE);
    }

    int grateid = getpid();
    
    // Because we assume that all cages are unaware of the existence of grate, cages will not handle the logic of `exec`ing 
    // grate, so we need to handle these two situations separately in grate. 
    // grate needs to fork in two situations: 
    // - the first is to fork and use its own cage; 
    // - the second is when there is still grate in the subsequent command line input. 
    // In the second case, we fork & exec the new grate and let the new grate handle the subsequent process.
    for (int i = 1; i < (argc < 3 ? argc : 3); i++) {
        pid_t pid = fork();
        if (pid < 0) {
            perror("fork failed");
            exit(EXIT_FAILURE);
        } else if (pid == 0) {
            // According to input format, the odd-numbered positions will always be grate, and even-numbered positions 
            // will always be cage.
            if (i % 2 != 0) {
                // Next one is cage, only cage set the register_handler
                int cageid = getpid();
                // Set the geteuid (syscallnum=51) of this cage to call this grate function geteuid_grate (func index=0)
                // Syntax of register_handler: <targetcage, targetcallnum, handlefunc_index_in_this_grate, this_grate_id>
                int ret = register_handler(cageid, 51, 0, grateid);
            }

            if ( execv(argv[i], &argv[i]) == -1) {
                perror("execv failed");
                exit(EXIT_FAILURE);
            }
        }
    }

    int status;
    while (wait(&status) > 0) {
        printf("[Grate | geteuid] terminated, status: %d\n", status);
    }
    
    return 0;
}
