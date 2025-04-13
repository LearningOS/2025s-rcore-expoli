//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next,
        suspend_current_and_run_next,
        get_current_task_call_syscall_id_time},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    // println!("[kernel] sys_trace: {} {} {}", _trace_request, _id, _data);
    match _trace_request {
        0 => {
            let ptr = _id as *const u8;
            let value = unsafe{ ptr.read() };
            // println!("[kernel] sys_trace: return read value: {}", value);
            value as isize
        },
        1 => {
            //   - 如果 trace_request 为 1，则 id 应被视作 *const u8 ，
            // 表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0。
            let ptr = _id as *mut u8;
            unsafe { *ptr = _data as u8 };
            0
        },
        2 => {
            let count =  get_current_task_call_syscall_id_time(_id);
            // println!("[kernel] sys_trace: return syscall count: {}", count);
            count
        },
        _ => -1,
    }
}
