#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>

void run_process(const char *filename) {
    pid_t pid = fork();

    if (pid < 0) {
        perror("fork failed");
        exit(1);
    } else if (pid == 0) {
        printf("Child process (PID: %d) executing %s...\n", getpid(), filename);

        execl(filename, filename, NULL);

        perror("execl failed");
        exit(1);
    }
}

int main() {
    run_process("hello.cwasm");

    run_process("pocadd.cwasm");

    int status;
    pid_t wpid;
    while ((wpid = wait(&status)) > 0) {
        if (WIFEXITED(status)) {
            printf("Child (PID: %d) exited with status %d\n", wpid, WEXITSTATUS(status));
        } else {
            printf("Child (PID: %d) did not exit normally.\n", wpid);
        }
    }

    return 0;
}
