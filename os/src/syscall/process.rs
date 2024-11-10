//! Process management syscalls
use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    mm::{translated_byte_buffer, MapPermission, VirtAddr},
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus, TASK_MANAGER,
    },
    timer::{get_time_us, MICRO_PER_SEC},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    let sec = us / MICRO_PER_SEC;
    let usec = us % MICRO_PER_SEC;

    let token = current_user_token();
    let ts_vec = translated_byte_buffer(token, ts as *const u8, core::mem::size_of::<TimeVal>());
    if ts_vec.len() == 0 {
        return -1;
    }
    let ts = unsafe { (ts_vec[0].as_ptr() as *mut TimeVal).as_mut().unwrap() };
    *ts = TimeVal {
        sec: sec,
        usec: usec,
    };
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let token = current_user_token();
    let ti_vec = translated_byte_buffer(token, ti as *const u8, core::mem::size_of::<TaskInfo>());
    if ti_vec.len() == 0 {
        return -1;
    }
    let ti = unsafe { (ti_vec[0].as_ptr() as *mut TaskInfo).as_mut().unwrap() };

    let inner = TASK_MANAGER.inner.exclusive_access();
    let task = &inner.tasks[inner.current_task];

    *ti = TaskInfo {
        status: task.task_status,
        syscall_times: [0; MAX_SYSCALL_NUM], // 需要额外实现系统调用计数
        time: get_time_us() / MICRO_PER_SEC,
    };
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    // 基本参数检查
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }

    let mut permission = MapPermission::U;
    if port & 1 != 0 {
        permission |= MapPermission::R;
    }
    if port & 2 != 0 {
        permission |= MapPermission::W;
    }
    if port & 4 != 0 {
        permission |= MapPermission::X;
    }

    let end = start + len;
    if end < start {
        return -1;
    }

    let start_va: VirtAddr = VirtAddr::from(start);
    let end_va: VirtAddr = VirtAddr::from(end);
    let start_vpn = start_va.floor();
    let end_vpn = end_va.ceil();

    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let task = &mut inner.tasks[current];

    // 检查是否与现有区域重叠
    for area in task.memory_set.areas.iter() {
        if area.vpn_range.get_start() < end_vpn && start_vpn < area.vpn_range.get_end() {
            return -1;
        }
    }

    task.memory_set
        .insert_framed_area(start_va, end_va, permission);
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    if start % PAGE_SIZE != 0 {
        return -1;
    }

    let end = start + len;
    if end < start {
        return -1;
    }

    let start_va: VirtAddr = VirtAddr::from(start);
    let end_va: VirtAddr = VirtAddr::from(end);

    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let task = &mut inner.tasks[current];

    // 查找对应的 MapArea
    if let Some(area) = task.memory_set.areas.iter_mut().find(|area| {
        area.vpn_range.get_start() == start_va.floor() && area.vpn_range.get_end() == end_va.floor()
    }) {
        area.unmap(&mut task.memory_set.page_table);
    } else {
        return -1;
    }
    0
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
