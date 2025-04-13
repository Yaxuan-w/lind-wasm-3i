// #include <stdio.h>
// #include <unistd.h>

// int main(int argc, char *argv[]) {
//     int ret = getuid();
//     printf("[Cage | getuid] getuid ret = %d\n", ret);
    
//     return 0;
// }

#include <stdio.h>
#include <unistd.h>
#include <pthread.h>

void* thread_func(void* arg) {
    int ret = getuid();
    printf("[Thread %ld | getuid] getuid ret = %d\n", (long)arg, ret);
    return NULL;
}

int main(int argc, char *argv[]) {
    pthread_t t1, t2;

    pthread_create(&t1, NULL, thread_func, (void*)1);
    pthread_create(&t2, NULL, thread_func, (void*)2);

    pthread_join(t1, NULL);
    pthread_join(t2, NULL);

    return 0;
}
