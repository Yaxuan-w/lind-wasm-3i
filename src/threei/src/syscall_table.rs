use super::threei::Raw_CallFunc;
use rawposix::syscalls::fs_calls::{
    brk_syscall, clock_gettime_syscall, close_syscall, dup2_syscall, dup_syscall, fcntl_syscall,
    mkdir_syscall, mmap_syscall, munmap_syscall, nanosleep_time64_syscall, open_syscall,
    pipe2_syscall, pipe_syscall, sbrk_syscall, write_syscall,
};
use rawposix::syscalls::sys_calls::{
    exec_syscall, exit_syscall, fork_syscall, getpid_syscall, wait_syscall, waitpid_syscall,
};

/// Will replace syscall number with Linux Standard after confirming the refactoring details
pub const SYSCALL_TABLE: &[(u64, Raw_CallFunc)] = &[
    (10, open_syscall),
    (11, close_syscall),
    (13, write_syscall),
    (21, mmap_syscall),
    (22, munmap_syscall),
    (24, dup_syscall),
    (25, dup2_syscall),
    (28, fcntl_syscall),
    (30, exit_syscall),
    (31, getpid_syscall),
    (66, pipe_syscall),
    (67, pipe2_syscall),
    (69, exec_syscall),
    (83, mkdir_syscall),
    (68, fork_syscall),
    (172, wait_syscall),
    (173, waitpid_syscall),
    (175, brk_syscall),
    (176, sbrk_syscall),
    (183, nanosleep_time64_syscall),
    (191, clock_gettime_syscall),
];
