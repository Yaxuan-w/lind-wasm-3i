use crate::rawposix::syscalls::fs_calls::{hello_syscall, write_syscall};
use crate::rawposix::syscalls::sys_calls::exit_syscall;
use crate::threei::threei::CallFunc;

/// Will replace syscall number with Linux Standard after confirming the refactoring details 
pub const SYSCALL_TABLE: &[(u64, CallFunc)] = &[
    (1, hello_syscall), // ONLY for testing purpose 
    (2, write_syscall),
    (30, exit_syscall),
    (171, fork_syscall),
    (69, exec_syscall),
];
