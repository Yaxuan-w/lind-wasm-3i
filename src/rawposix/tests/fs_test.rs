use threei::cage::*;
use threei::fdtables;
use threei::rawposix::vmmap::*;
use threei::threei::{threei::*, threeiconstant};

use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, instrument};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};

const FDKIND_KERNEL: u32 = 0;

/// Helper functions:
/// Create a test cage for testing purpose
fn simple_init_cage(cageid: u64) {
    println!("simple_init_cage called with cageid: {}", cageid);
    // fdtables::register_close_handlers(FDKIND_KERNEL, fdtables::NULL_FUNC, kernel_close);
    let cage = Cage {
        cageid: cageid,
        cwd: RwLock::new(Arc::new(PathBuf::from("/"))),
        parent: 1,
        gid: AtomicI32::new(-1),
        uid: AtomicI32::new(-1),
        egid: AtomicI32::new(-1),
        euid: AtomicI32::new(-1),
        main_threadid: AtomicU64::new(0),
        zombies: RwLock::new(vec![]),
        child_num: AtomicU64::new(0),
        vmmap: RwLock::new(Vmmap::new()),
    };
    add_cage(cage);
    fdtables::init_empty_cage(cageid);
    println!("ADDED cage {:?} to fdtable", cageid);
    // Set the first 3 fd to STDIN / STDOUT / STDERR
    // STDIN
    // let dev_null = CString::new("/home/lind/lind_project/src/safeposix-rust/tmp/dev/null").unwrap();
    fdtables::get_specific_virtual_fd(cageid, 0, FDKIND_KERNEL, 0, false, 0).unwrap();
    // STDOUT
    fdtables::get_specific_virtual_fd(cageid, 1, FDKIND_KERNEL, 1, false, 0).unwrap();
    // STDERR
    fdtables::get_specific_virtual_fd(cageid, 2, FDKIND_KERNEL, 2, false, 0).unwrap();
}

/// Simple testing make functionality
#[test]
fn test_make_syscall() {
    let cageid = 42;
    simple_init_cage(cageid);
    let cage2id = 40;
    simple_init_cage(cage2id);
    let reg_result = register_handler(
        0,       // Unused, kept for syscall convention
        cageid,  // target cageid 42
        1,       // target syscall: hello
        0,       // Unused
        2,       // self syscall: write
        cage2id, // self cageid 40
        0, 0, 0, 0, 0, 0, 0, 0, // Unused
    );
    assert_eq!(
        reg_result, 0,
        "register_handler did not return the expected result"
    );

    // test make in case of same cageid
    let result = make_syscall(
        cage2id, //40
        2, 1, // hello syscall
        cage2id, 0, 0, 0, 0, 0, 0,
    );
    assert_eq!(result, 0, "make_syscall did not return the expected result");

    // test make in case of different cageid
    let result2 = make_syscall(
        cage2id, // 40
        2, 1,      // hello
        cageid, // 42
        0, 0, 0, 0, 0, 0,
    );

    assert_eq!(
        result2, 0,
        "make_syscall second time did not return the expected result"
    );

    testing_remove_all();
}

/// Test if we can successfully copy syscall handler -- the value of them is correct
/// TODO:
/// - Test if copied handler could work as expectation
#[test]
fn test_copy_handler() {
    let cageid = 41;
    simple_init_cage(cageid);
    let cage2id = 40;
    simple_init_cage(cage2id);
    let cage3id = 80;
    simple_init_cage(cage3id);

    // Register cage1 handler
    let reg_result = register_handler(
        0,       // Unused, kept for syscall convention
        cage2id, // target cageid 40
        1,       // Syscall number or match-all indicator
        0,       // Unused
        2,       //
        cageid,  // self cageid 41
        0, 0, 0, 0, 0, 0, 0, 0, // Unused
    );
    assert_eq!(
        reg_result, 0,
        "register_handler did not return the expected result"
    );

    let copy_result =
        copy_handler_table_to_cage(0, cage3id, cageid, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
    assert_eq!(
        copy_result, 0,
        "copy_handler didn't return the expected results"
    );

    // The block of code is enclosed within curly braces to explicitly scope the lock on the `HANDLERTABLE`,
    // which ensures that the lock is released as soon as the operation within the block is completed.
    {
        let handler_table = HANDLERTABLE.lock().unwrap();

        // Test if cage1 entry exists
        assert!(
            handler_table.contains_key(&cageid),
            "HANDLERTABLE does not contain cage1 entries"
        );

        // Test if cage3 entry exists
        assert!(
            handler_table.contains_key(&cage3id),
            "HANDLERTABLE does not contain cage3 entries"
        );

        // Compare cage1 and cage3 entries
        let cage1_entries = handler_table.get(&cageid).unwrap();
        let cage3_entries = handler_table.get(&cage3id).unwrap();

        for (callnum, cage1_handler) in cage1_entries.iter() {
            let cage3_handler = cage3_entries
                .get(callnum)
                .expect("Handler not found in cage3");

            let cage1_call = cage1_handler.lock().unwrap();
            let cage3_call = cage3_handler.lock().unwrap();

            assert_eq!(
                cage1_call.defaultcallfunc, cage3_call.defaultcallfunc,
                "DefaultCallFunc mismatch for callnum {}",
                callnum
            );
            assert_eq!(
                cage1_call.thiscalltable, cage3_call.thiscalltable,
                "ThisCallTable mismatch for callnum {}",
                callnum
            );
        }
    }

    // TODO:
    // Add test for removing one entry and check again

    testing_remove_all();
}

/// Test exit
#[test]
fn test_exit() {
    let cageid = 41;
    simple_init_cage(cageid);
    let cage2id = 40;
    simple_init_cage(40);
    thread::sleep(Duration::from_secs(10));
    // Register cage1 handler: cage1/write --> cage2/write
    let reg_result = register_handler(
        0, cage2id, // target cageid for cage2: 40
        2,       // write syscall
        0,       // Unused
        2,       // write syscall
        cageid,  // self cageid for cage1: 41
        0, 0, 0, 0, 0, 0, 0, 0, // Unused
    );
    assert_eq!(
        reg_result, 0,
        "register_handler did not return the expected result"
    );

    // Register cage1 handler: cage1/write --> cage2/hello
    let reg2_result = register_handler(
        0, cage2id, // target cageid for cage2: 40
        1,       // hello syscall
        0,       // Unused
        2,       // write syscall
        cageid,  // self cageid for cage1: 41
        0, 0, 0, 0, 0, 0, 0, 0, // Unused
    );
    assert_eq!(
        reg2_result, 0,
        "register_handler did not return the expected result"
    );

    // Call write from cage1 to cage2
    let make_result = make_syscall(
        cageid, 2, 1, // hello syscall
        cage2id, 0, 0, 0, 0, 0, 0,
    );
    assert_eq!(
        make_result, 0,
        "make_syscall did not return the expected result"
    );

    // Initialize tracing subscriber with a custom layer
    tracing_subscriber::registry()
        .with(fmt::layer().with_timer(fmt::time::uptime()))
        .init();
    let start = Instant::now();

    // Exit cage1
    trigger_harsh_cage_exit(
        cageid, 0, // random exit type for testing purpose
    );

    let duration = start.elapsed();
    println!("trigger_harsh_cage_exit entire completed in {:?}", duration);

    // Call write from cage1 to cage2 again, should fail
    let make_result = make_syscall(
        cageid, 2, // write syscall
        1, cage2id, 0, 0, 0, 0, 0, 0,
    );
    assert_eq!(
        make_result,
        threeiconstant::ELINDESRCH as i32,
        "make_syscall did not return the expected result"
    );

    // Check if HANDLERTABLE is empty
    let handler_table = HANDLERTABLE.lock().unwrap();
    assert!(
        handler_table.is_empty(),
        "HANDLERTABLE is not empty after exit, remaining entries: {:?}",
        *handler_table
    );
}

/// Test basic pipe creation and operations
#[test]
fn test_pipe() {
    let cageid = 50;
    simple_init_cage(cageid);

    let mut pipe_fds = [-1; 2];
    let pipe_result = pipe(cageid, pipe_fds.as_mut_ptr());
    assert_eq!(pipe_result, 0, "pipe creation failed");
    assert!(pipe_fds[0] >= 0, "read end of pipe is invalid");
    assert!(pipe_fds[1] >= 0, "write end of pipe is invalid");

    // Test basic read/write through pipe
    let test_data = b"Hello, pipe!";
    let write_result = write(
        cageid,
        pipe_fds[1],
        test_data.as_ptr() as *const c_void,
        test_data.len(),
    );
    assert_eq!(
        write_result as usize,
        test_data.len(),
        "write to pipe failed"
    );

    let mut read_buffer = vec![0u8; test_data.len()];
    let read_result = read(
        cageid,
        pipe_fds[0],
        read_buffer.as_mut_ptr() as *mut c_void,
        read_buffer.len(),
    );
    assert_eq!(
        read_result as usize,
        test_data.len(),
        "read from pipe failed"
    );
    assert_eq!(
        &read_buffer, test_data,
        "read data doesn't match written data"
    );

    testing_remove_all();
}

/// Test pipe2 with various flags
#[test]
fn test_pipe2() {
    let cageid = 51;
    simple_init_cage(cageid);

    let mut pipe2_fds = [-1; 2];
    let pipe2_result = pipe2(cageid, pipe2_fds.as_mut_ptr(), O_NONBLOCK);
    assert_eq!(pipe2_result, 0, "pipe2 creation failed");
    assert!(pipe2_fds[0] >= 0, "read end of pipe2 is invalid");
    assert!(pipe2_fds[1] >= 0, "write end of pipe2 is invalid");

    // Test non-blocking behavior
    let large_buffer = vec![0u8; 65536]; // Size larger than pipe buffer
    let write_result = write(
        cageid,
        pipe2_fds[1],
        large_buffer.as_ptr() as *const c_void,
        large_buffer.len(),
    );
    assert!(
        write_result < large_buffer.len() as isize,
        "Non-blocking write should not block"
    );

    testing_remove_all();
}

/// Test read syscall behavior
#[test]
fn test_read() {
    let cageid = 52;
    simple_init_cage(cageid);

    let mut pipe_fds = [-1; 2];
    pipe(cageid, pipe_fds.as_mut_ptr());

    // Test reading with different buffer sizes
    let test_data = b"Testing read syscall";
    write(
        cageid,
        pipe_fds[1],
        test_data.as_ptr() as *const c_void,
        test_data.len(),
    );

    // Test partial read
    let mut small_buffer = vec![0u8; 5];
    let read_result = read(
        cageid,
        pipe_fds[0],
        small_buffer.as_mut_ptr() as *mut c_void,
        small_buffer.len(),
    );
    assert_eq!(read_result as usize, 5, "partial read failed");
    assert_eq!(&small_buffer, &test_data[..5], "partial read data mismatch");

    testing_remove_all();
}

/// Test close syscall behavior
#[test]
fn test_close() {
    let cageid = 53;
    simple_init_cage(cageid);

    let mut pipe_fds = [-1; 2];
    pipe(cageid, pipe_fds.as_mut_ptr());

    // Test closing read end
    let close_read_result = close(cageid, pipe_fds[0]);
    assert_eq!(close_read_result, 0, "closing read end failed");

    // Test closing write end
    let close_write_result = close(cageid, pipe_fds[1]);
    assert_eq!(close_write_result, 0, "closing write end failed");

    // Test double close (should fail)
    let double_close_result = close(cageid, pipe_fds[0]);
    assert!(double_close_result < 0, "double close should fail");

    // Test closing invalid fd
    let invalid_fd_result = close(cageid, 99999);
    assert!(invalid_fd_result < 0, "closing invalid fd should fail");

    testing_remove_all();
}
