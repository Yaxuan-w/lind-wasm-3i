#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <signal.h>
#include <string.h>

/**
 * 我们假设:
 *   - "cage.cwasm" 会在接收到 SIGUSR1 信号后才正式开始跑它的业务逻辑；
 *   - "pocadd.cwasm" 会收到 cage_pid，并在 main() 里做完一些操作后保持运行不退出。
 */

int main() {
    pid_t cage_pid = fork();
    if (cage_pid < 0) {
        perror("fork for cage failed");
        exit(1);
    } else if (cage_pid == 0) {
        // 子进程：执行 cage.cwasm，传入一个特殊参数 "wait"
        printf("[grate-loader] Child cage (PID: %d) executing cage.cwasm in 'wait' mode...\n", getpid());
        
        // 让 cage.cwasm 自行解析 argv[1] == "wait" 并阻塞或等待信号
        execl("./cage.cwasm", "cage.cwasm", "wait", NULL);
        
        // 如果 execl 返回，一定是出错了
        perror("execl for cage.cwasm failed");
        exit(1);
    }

    // 父进程继续
    printf("[grate-loader] cage.cwasm started (PID: %d), but it should be waiting for a signal...\n", cage_pid);

    // 现在启动 pocadd.cwasm，把 cage_pid 作为参数传过去
    pid_t pocadd_pid = fork();
    if (pocadd_pid < 0) {
        perror("fork for pocadd failed");
        exit(1);
    } else if (pocadd_pid == 0) {
        // 子进程：执行 pocadd.cwasm，传入 cage_pid
        char pid_str[32];
        snprintf(pid_str, sizeof(pid_str), "%d", cage_pid);

        printf("[grate-loader] Child pocadd (PID: %d) executing pocadd.cwasm, with cage_pid = %s\n", getpid(), pid_str);

        execl("./pocadd.cwasm", "pocadd.cwasm", pid_str, NULL);

        // 如果 execl 返回，一定是出错了
        perror("execl for pocadd.cwasm failed");
        exit(1);
    }

    // 父进程到这里，pocadd 进程已经开始运行，并拿到 cage_pid。
    // pocadd 里可以做 register_handler(...) 等操作。
    // 这里我们并不马上等它退出，因为需求是：pocadd 继续活着，直到 cage 结束后才退出。

    printf("[grate-loader] pocadd.cwasm started (PID: %d). We'll let it do its initialization...\n", pocadd_pid);

    // 简单做法：手动等待用户按下回车，模拟“pocadd 已经完成 register_handler 初始化”
    // 当然你可以通过管道、共享内存、socket、信号等做更自动化的同步。
    printf("[grate-loader] Press <Enter> after pocadd has done its registration...\n");
    getchar();

    // 现在让 cage 真正开始跑它的逻辑，给它发 SIGUSR1
    printf("[grate-loader] Sending SIGUSR1 to cage (PID: %d) to wake it up.\n", cage_pid);
    kill(cage_pid, SIGUSR1);

    // 等 cage 跑完
    int status;
    pid_t w = waitpid(cage_pid, &status, 0);
    if (w == -1) {
        perror("waitpid for cage failed");
    } else {
        if (WIFEXITED(status)) {
            printf("[grate-loader] cage.cwasm (PID: %d) exited with status %d.\n", w, WEXITSTATUS(status));
        } else {
            printf("[grate-loader] cage.cwasm (PID: %d) did not exit normally.\n", w);
        }
    }

    // cage 结束后，再让 pocadd 退出（如果你需要一直让它跑，可以不 kill；具体看你需求）
    printf("[grate-loader] cage done. Now sending SIGTERM to pocadd (PID: %d) to end.\n", pocadd_pid);
    kill(pocadd_pid, SIGTERM);

    // 再等 pocadd
    w = waitpid(pocadd_pid, &status, 0);
    if (w == -1) {
        perror("waitpid for pocadd failed");
    } else {
        if (WIFEXITED(status)) {
            printf("[grate-loader] pocadd.cwasm (PID: %d) exited with status %d.\n", w, WEXITSTATUS(status));
        } else {
            printf("[grate-loader] pocadd.cwasm (PID: %d) did not exit normally.\n", w);
        }
    }

    printf("[grate-loader] All done.\n");
    return 0;
}
// pocadd.cwasm (伪代码)
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

int main(int argc, char *argv[]) {
    if (argc > 1) {
        int cage_pid = atoi(argv[1]);
        printf("[pocadd] My PID: %d, got cage_pid: %d.\n", getpid(), cage_pid);

        // 这里做你需要的 register_handler(...) 之类的初始化操作
        printf("[pocadd] Doing register_handler(...) with cage_pid...\n");
        // register_handler(..., cage_pid);
        
        // 如果你希望 pocadd 不要退出，就让它一直阻塞或循环:
        printf("[pocadd] register_handler done, I will keep running until forcibly terminated.\n");

        // 简单持续运行5秒示例（你可以改成 while(1) 或更复杂逻辑）
        for (int i = 0; i < 15; i++) {
            printf("[pocadd] Running... (i=%d)\n", i);
            sleep(1);
        }
        printf("[pocadd] Exiting normally.\n");
    } else {
        // 如果没给参数，就直接退出
        printf("[pocadd] No cage_pid provided.\n");
    }

    return 0;
}
// cage.cwasm (伪代码)
#include <stdio.h>
#include <signal.h>
#include <string.h>
#include <unistd.h>

static int g_ready = 0;

void handle_sigusr1(int signo) {
    g_ready = 1;
    // 也可以直接在这里把 pause() 打断
}

int main(int argc, char *argv[]) {
    if (argc > 1 && strcmp(argv[1], "wait") == 0) {
        // 注册 SIGUSR1 的处理函数
        signal(SIGUSR1, handle_sigusr1);
        printf("[cage] Starting in 'wait' mode. Waiting for SIGUSR1...\n");

        // 阻塞直到收到 SIGUSR1
        while (!g_ready) {
            pause();
        }
        printf("[cage] Received SIGUSR1, now proceed with real logic.\n");
    }

    // 这里就是 cage 真实要做的事
    for (int i = 0; i < 5; i++) {
        printf("[cage] Doing real job step %d...\n", i);
        sleep(1);
    }
    printf("[cage] Done.\n");
    return 0;
}
