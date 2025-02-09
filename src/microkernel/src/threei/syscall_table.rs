use crate::rawposix::syscalls::fs_calls::{hello_syscall, write_syscall, open_syscall, mkdir_syscall};
use crate::rawposix::syscalls::sys_calls::{exit_syscall, exec_syscall, fork_syscall};
use crate::threei::threei::CallFunc;

/// Will replace syscall number with Linux Standard after confirming the refactoring details 
pub const SYSCALL_TABLE: &[(u64, CallFunc)] = &[
    (0, hello_syscall), // ONLY for testing purpose 
    (1, write_syscall),
    (2, open_syscall),
    (30, exit_syscall),
    (83, mkdir_syscall),
    (171, fork_syscall),
    (69, exec_syscall),
];

