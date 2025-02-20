#![allow(dead_code)]

use anyhow::Result;
use threei::threei::{make_syscall, threei_test_func, MyCallback};
use wasmtime_lind_multi_process::{get_memory_base, LindHost, clone_constants::CloneArgStruct};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use wasmtime::{Caller, Func};

// -------------- AW --------------
use wasmtime::Val;
// -------------- AW --------------

// lind-common serves as the main entry point when lind_syscall. Any syscalls made in glibc would reach here first,
// then the syscall would be dispatched into rawposix, or other crates under wasmtime, depending on the syscall, to perform its job

#[derive(Clone)]
// stores some attributes associated with current runnning wasm instance (i.e. cage)
// each cage has its own lind-common context 
pub struct LindCommonCtx {
    // process id attached to the lind-common context, should be same as cage id
    pid: i32,
    
    // next cage id, shared between all lind-common context instance (i.e. all cages)
    next_cageid: Arc<AtomicU64>,
}

impl LindCommonCtx {
    // create a new lind-common context, should only be called once for then entire runtime
    pub fn new(next_cageid: Arc<AtomicU64>) -> Result<Self> {
        // cage id starts from 1
        let pid = 1;
        Ok(Self { pid, next_cageid })
    }

    // create a new lind-common context with pid provided, used by exec syscall
    pub fn new_with_pid(pid: i32, next_cageid: Arc<AtomicU64>,
        ) -> Result<Self> {
        Ok(Self { pid, next_cageid })
    }

    // entry point for lind_syscall in glibc, dispatching syscalls to rawposix or wasmtime
    pub fn lind_syscall<T: LindHost<T, U> + Clone + Send + 'static + std::marker::Sync, U: Clone + Send + 'static + std::marker::Sync>
                        (&self, call_number: u32, call_name: u64, caller: &mut Caller<'_, T>, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64, arg6: u64) -> i32 {
        let start_address = get_memory_base(&caller);
        match call_number as i32 {
            // clone syscall
            171 => {
                let clone_args = unsafe { &mut *((arg1 + start_address) as *mut CloneArgStruct) };
                clone_args.child_tid += start_address;
                wasmtime_lind_multi_process::clone_syscall(caller, clone_args)
            }
            // exec syscall
            69 => {
                wasmtime_lind_multi_process::exec_syscall(caller, arg1 as i64, arg2 as i64, arg3 as i64)
            }
            // exit syscall
            30 => {
                wasmtime_lind_multi_process::exit_syscall(caller, arg1 as i32)
            }
            // other syscalls goes into rawposix
            _ => {
                make_syscall(
                    self.pid as u64,
                    call_number as u64,
                    self.pid as u64, // Set target_cageid same with self_cageid by defualt 
                    arg1, 
                    self.pid as u64,
                    arg2,
                    self.pid as u64,
                    arg3, 
                    self.pid as u64,
                    arg4,
                    self.pid as u64,
                    arg5,
                    self.pid as u64,
                    arg6,
                    self.pid as u64,
                )
            }
        }
    }

    // setjmp call. This function needs to be handled within wasmtime, but it is not an actual syscall so we use a different routine from lind_syscall
    pub fn lind_setjmp<T: LindHost<T, U> + Clone + Send + 'static + std::marker::Sync, U: Clone + Send + 'static + std::marker::Sync>
                        (&self, caller: &mut Caller<'_, T>, jmp_buf: u32) -> i32 {
        wasmtime_lind_multi_process::setjmp_call(caller, jmp_buf)
    }

    // longjmp call. This function needs to be handled within wasmtime, but it is not an actual syscall so we use a different routine from lind_syscall
    pub fn lind_longjmp<T: LindHost<T, U> + Clone + Send + 'static + std::marker::Sync, U: Clone + Send + 'static + std::marker::Sync>
                        (&self, caller: &mut Caller<'_, T>, jmp_buf: u32, retval: i32) -> i32 {
        wasmtime_lind_multi_process::longjmp_call(caller, jmp_buf, retval)
    }

    // get current process id/cageid
    // currently unused interface but may be useful in the future
    pub fn getpid(&self) -> i32 {
        self.pid
    }

    // return the next avaliable cageid (cageid increment sequentially)
    fn next_cage_id(&self) -> Option<u64> {
        match self
            .next_cageid
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| match v {
                ..=0x1ffffffe => Some(v + 1),
                _ => None,
            }) {
            Ok(v) => Some(v + 1),
            Err(_) => None,
        }
    }

    // fork a new lind-common context, used by clone syscall
    pub fn fork(&self) -> Self {
        // cageid is automatically incremented here
        let next_pid = self.next_cage_id().unwrap();

        let forked_ctx = Self {
            pid: next_pid as i32,
            next_cageid: self.next_cageid.clone(),
        };

        return forked_ctx;
    }

    // -------------- AW --------------
    pub fn wasmtime_test_func<'a, T: LindHost<T, U> + Clone + Send + 'static + std::marker::Sync, U: Clone + Send + 'static + std::marker::Sync>
            (&self, caller: &'a mut Caller<'a, T>) {
        let func = match caller.get_export("c_test_func") {
            Some(wasmtime::Extern::Func(f)) => f,
            _ => panic!("Function not found in Wasm"),
        };
    
        // Send to `threei_test_func`
        threei_test_func(Box::new(move || -> i32 {
            let mut store = caller.as_context_mut();
    
            // Define return value
            let mut results = [Val::I32(0)];
    
            match func.call(&mut store, &[], &mut results) {
                Ok(_) => {
                    if let Val::I32(value) = results[0] {
                        value
                    } else {
                        panic!("Unexpected return type from Wasm function");
                    }
                }
                Err(e) => {
                    eprintln!("Error calling Wasm function: {:?}", e);
                    -1
                }
            }
        }));
    }
    // -------------- AW --------------
}

// function to expose the handler to wasm module
// linker: wasmtime's linker to link the imported function to the actual function definition
// get_cx: function to retrieve LindCommonCtx from caller
pub fn add_to_linker<T: LindHost<T, U> + Clone + Send + 'static + std::marker::Sync, U: Clone + Send + 'static + std::marker::Sync>(
    linker: &mut wasmtime::Linker<T>,
    get_cx: impl Fn(&T) -> &LindCommonCtx + Send + Sync + Copy + 'static,
) -> anyhow::Result<()> {
    // -------------- AW --------------
    // Grate's tmp call num to indicate calling from 3i
    linker.func_wrap(
        "lind",
        "c_test_func",
        move |caller: Caller<'_, T>| {
            let host = caller.data().clone();
            let ctx = get_cx(&host);

            let mut caller = caller;
            ctx.wasmtime_test_func(&mut caller)
        },
    )?;
    // -------------- AW --------------
    // attach lind_syscall to wasmtime
    linker.func_wrap(
        "lind",
        "lind-syscall",
        move |mut caller: Caller<'_, T>, call_number: u32, call_name: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64, arg6: u64| -> i32 {
            let host = caller.data().clone();
            let ctx = get_cx(&host);

            ctx.lind_syscall(call_number, call_name, &mut caller, arg1, arg2, arg3, arg4, arg5, arg6)
        },
    )?;

    // attach setjmp to wasmtime
    linker.func_wrap(
        "lind",
        "lind-setjmp",
        move |mut caller: Caller<'_, T>, jmp_buf: i32| -> i32 {
            let host = caller.data().clone();
            let ctx = get_cx(&host);
            
            ctx.lind_setjmp(&mut caller, jmp_buf as u32)
        },
    )?;

    // attach longjmp to wasmtime
    linker.func_wrap(
        "lind",
        "lind-longjmp",
        move |mut caller: Caller<'_, T>, jmp_buf: i32, retval: i32| -> i32 {
            let host = caller.data().clone();
            let ctx = get_cx(&host);

            ctx.lind_longjmp(&mut caller, jmp_buf as u32, retval)
        },
    )?;

    Ok(())
}
