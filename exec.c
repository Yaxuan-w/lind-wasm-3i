#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>

int main() {
    pid_t pid = fork();

    if (pid < 0) {
        perror("fork failed");
        exit(1);
    } else if (pid == 0) {
        printf("Child process (PID: %d) executing hello...\n", getpid());

        execl("hello.cwasm", "hello.cwasm", NULL);

        // Only execute when execl fails 
        perror("execl failed");
        exit(1);
    } else {
        int status;
        waitpid(pid, &status, 0);
        printf("Child process exited with status %d\n", WEXITSTATUS(status));
    }

    return 0;
}

