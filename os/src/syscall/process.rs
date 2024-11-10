//! Process management syscalls
use crate::{
    config::{MAX_SYSCALL_NUM, PAGE_SIZE},
    mm::{translated_byte_buffer, MapPermission, VirtAddr},
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus, TASK_MANAGER,
    },
    timer::get_time_us,
};
//use crate::mm::address::VPNRange;  
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
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
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
    trace!("kernel: sys_get_time");
    let token = current_user_token();

    // 2. 将用户空间的虚拟地址转换为内核可访问的缓冲区
    let ts_buffers = translated_byte_buffer(
        token,                           // 用户的页表 token
        ts as *const u8,                 // 转换为字节指针
        core::mem::size_of::<TimeVal>(), // TimeVal 结构体的大小
    );

    let us = get_time_us();
    let ts_struct = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    // 5. 写入用户空间
    unsafe {
        *(ts_buffers[0].as_ptr() as *mut TimeVal) = ts_struct;
    }
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let token = current_user_token();
    let ti_buffers =
        translated_byte_buffer(token, ti as *const u8, core::mem::size_of::<TaskInfo>());

    let task_info = TASK_MANAGER.get_current_task_info();
    unsafe {
        // 写入用户空间
        *(ti_buffers[0].as_ptr() as *mut TaskInfo) = task_info;
    }
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
        // 检查参数合法性
    if start % PAGE_SIZE != 0 {  // 起始地址必须页对齐
        return -1;
    }
    if port & !0x7 != 0 {  // port高位必须为0
        return -1;
    }
    if port & 0x7 == 0 {   // port至少要有一个权限位
        return -1;
    }
    
    let mut permission = MapPermission::U;
    if port & 1 != 0 { permission |= MapPermission::R; }
    if port & 2 != 0 { permission |= MapPermission::W; }
    if port & 4 != 0 { permission |= MapPermission::X; }

    // 计算实际长度(页对齐)
    let len = (len + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    if len == 0 { return 0; }


    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let task = &mut inner.tasks[current];
    let memory_set = &mut task.memory_set;

    // 检查目标区域是否已被映射
    let start_va: VirtAddr = VirtAddr::from(start);
    let end_va: VirtAddr = VirtAddr::from(start + len);
    // 检查地址范围是否合法
    for area in memory_set.areas.iter() {
        let area_start = area.vpn_range.get_start();
        let area_end = area.vpn_range.get_end();
        if !(end_va.floor() <= area_start || start_va.ceil() >= area_end) {
            return -1;
        }
    }

    // 建立映射
    memory_set.insert_framed_area(start_va, end_va, permission);
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    // 参数检查
    if start % PAGE_SIZE != 0 {
        return -1;
    }

    // 计算实际长度(页对齐)
    let len = (len + PAGE_SIZE - 1) & !(PAGE_SIZE - 1);
    if len == 0 { return 0; }

    let start_va: VirtAddr = VirtAddr::from(start);
    let end_va: VirtAddr = VirtAddr::from(start + len);


    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current = inner.current_task;
    let task = &mut inner.tasks[current];
    let memory_set = &mut task.memory_set;
  // 找到要解除映射的区域
    let mut found = false;
    let mut idx = 0;
    for (i, area) in memory_set.areas.iter().enumerate() {
        if area.vpn_range.get_start() == start_va.floor() && 
           area.vpn_range.get_end() == end_va.ceil() {
            found = true;
            idx = i;
            break;
        }
    }

    if !found {
        return -1;
    }

    // 解除映射
    let mut area = memory_set.areas.remove(idx);
    area.unmap(&mut memory_set.page_table);
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
