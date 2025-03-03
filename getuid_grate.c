#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <register_handler.h>
#include <sys/types.h>
#include <sys/wait.h>

int uid;  
typedef int (*func_ptr_t)(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);
int getuid_grate(uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t, uint64_t);

func_ptr_t func_array[1] = {getuid_grate};

int pass_fptr_to_wt(uint64_t index, uint64_t cageid, uint64_t arg1, uint64_t arg1cage, uint64_t arg2, uint64_t arg2cage, uint64_t arg3, uint64_t arg3cage, uint64_t arg4, uint64_t arg4cage, uint64_t arg5, uint64_t arg5cage, uint64_t arg6, uint64_t arg6cage) {
    if (index < 0 || index >= 2) {
        fprintf(stderr, "Invalid index: %llu\n", index);
        return -1; 
    }
    
    return func_array[index](cageid, arg1, arg1cage, arg2, arg2cage, arg3, arg3cage, arg4, arg4cage, arg5, arg5cage, arg6, arg6cage);
}

int getuid_grate(uint64_t cageid, uint64_t arg1, uint64_t arg1cage, uint64_t arg2, uint64_t arg2cage, uint64_t arg3, uint64_t arg3cage, uint64_t arg4, uint64_t arg4cage, uint64_t arg5, uint64_t arg5cage, uint64_t arg6, uint64_t arg6cage) {
    printf("[grate | getuid] getuid: %d", uid);
    return uid;
}

int main(int argc, char *argv[]) {
    // if (argc < 3) {
    //     fprintf(stderr, "Usage: %s <default_uid> <grate/cage> [...]\n", argv[0]);
    //     exit(EXIT_FAILURE);
    // }

    uid = atoi(argv[1]);
    int grateid = getpid();

    printf("[Grate | getuid] setted uid: %d\n", uid);
    
    pid_t cageid = fork();
    if (cageid < 0) {
        perror("fork failed");
        exit(EXIT_FAILURE);
    }else if (cageid == 0) {  
        // <targetcage, targetcallnum, handlefunc_index_in_this_grate, this_grate_id>
        int ret = register_handler(cageid, 50, 1, grateid);
        if ( execl("getuid.cwasm", "getuid.cwasm", NULL) == -1) {
            perror("execl failed");
            exit(EXIT_FAILURE);
        }
    } 

    int status;
    if (waitpid(cageid, &status, 0) == -1) {
        perror("waitpid failed");
        exit(EXIT_FAILURE);
    }
    printf("Grate terminated, status: %d\n", status);
    
    return 0;
}
