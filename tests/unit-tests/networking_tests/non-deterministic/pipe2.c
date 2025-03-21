#undef _GNU_SOURCE
#define _GNU_SOURCE

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <unistd.h>

int main(void) {
  char str[4096] = {0};
  int ret, fd[2];

  if (pipe2(fd, 0) < 0) {
    perror("pipe2()");
    exit(EXIT_FAILURE);
  }
  printf("pipe2() ret: [%d, %d]\n", fd[0], fd[1]);
  fflush(stdout);

  if ((ret = write(fd[1], "hi\n", 3)) < 0) {
    printf("write(): %s\n", strerror(errno));
    exit(EXIT_FAILURE);
  }
  printf("write() ret: %d\n", ret);
  fflush(stdout);

  if ((ret = read(fd[0], str, 3)) < 0) {
    printf("read(): %s\n", strerror(errno));
    exit(EXIT_FAILURE);
  }
  printf("read() ret: %d\n", ret);
  fflush(stdout);

  for (size_t i = 0; i < sizeof fd / sizeof *fd; i++) {
    if (close(fd[i]) < 0) {
      perror("close()");
      exit(EXIT_FAILURE);
    }
  }
  puts(str);

  return 0;
}

