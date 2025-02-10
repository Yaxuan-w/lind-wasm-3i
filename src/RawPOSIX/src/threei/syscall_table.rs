use crate::syscalls::fs_calls::{hello_syscall, write_syscall, open_syscall, mkdir_syscall, mmap_syscall, munmap_syscall, brk_syscall, sbrk_syscall};
use crate::syscalls::sys_calls::{exit_syscall, exec_syscall, fork_syscall};
use crate::threei::threei::CallFunc;

/// Will replace syscall number with Linux Standard after confirming the refactoring details 
pub const SYSCALL_TABLE: &[(u64, CallFunc)] = &[
    (1, hello_syscall), // ONLY for testing purpose 
    (13, write_syscall),
    (10, open_syscall),
    (21, mmap_syscall),
    (22, munmap_syscall),
    (30, exit_syscall),
    (69, exec_syscall),
    (83, mkdir_syscall),
    (171, fork_syscall),
    (175, brk_syscall),
    (176, sbrk_syscall),
];

