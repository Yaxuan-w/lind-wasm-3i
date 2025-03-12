use rawposix::syscalls::fs_calls::{
    brk_syscall, mkdir_syscall, mmap_syscall, munmap_syscall, open_syscall, dup_syscall,
    sbrk_syscall, write_syscall, clock_gettime_syscall, fcntl_syscall, dup2_syscall,
    nanosleep_time64_syscall, stat_syscall, fstat_syscall, ioctl_syscall, close_syscall,
    read_syscall, lseek_syscall,
};
use rawposix::syscalls::sys_calls::{
    exec_syscall, exit_syscall, fork_syscall, getpid_syscall, wait_syscall, waitpid_syscall, geteuid_syscall,
    gethostname_syscall, getppid_syscall, getuid_syscall, getegid_syscall, getgid_syscall,
};

use super::threei::Raw_CallFunc;

/// Will replace syscall number with Linux Standard after confirming the refactoring details
pub const SYSCALL_TABLE: &[(u64, Raw_CallFunc)] = &[
    (9, stat_syscall),
    (10, open_syscall),
    (11, close_syscall),
    (12, read_syscall),
    (13, write_syscall),
    (14, lseek_syscall),
    (15, ioctl_syscall),
    (17, fstat_syscall),
    (21, mmap_syscall),
    (22, munmap_syscall),
    (24, dup_syscall),
    (25, dup2_syscall),
    (28, fcntl_syscall),
    (29, getppid_syscall),
    (30, exit_syscall),
    (31, getpid_syscall),
    (50, getuid_syscall),
    (51, geteuid_syscall),
    (52, getgid_syscall),
    (53, getegid_syscall),
    (69, exec_syscall),
    (83, mkdir_syscall),
    (68, fork_syscall),
    (125, gethostname_syscall),
    (172, wait_syscall),
    (173, waitpid_syscall),
    (175, brk_syscall),
    (176, sbrk_syscall),
    (183, nanosleep_time64_syscall),
    (191, clock_gettime_syscall),
];
