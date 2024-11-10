# 实现的功能

我实现了 sys_task_info 系统调用，该功能用于获取当前任务的运行信息。主要包括三个部分：
获取任务状态：通过 TaskManager 访问当前任务的 TaskStatus
记录系统调用次数：在每次系统调用时更新计数器数组
计算任务运行时间：从任务开始到当前的总运行时间
这些信息被填充到传入的 TaskInfo 结构体中，供用户程序查询任务的运行状态。

# 简答题

## 1.

ch2b_bad_address.rs: 尝试访问非法地址
ch2b_bad_instructions.rs: 尝试执行特权指令
ch2b_bad_register.rs: 尝试访问特权寄存器

版本：[rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0

## 2.

### 1.

a0 指向 TrapContext 的地址(与sp相同)，也即当前任务的内核态

第一种场景：当操作系统第一次启动一个用户程序时，会通过TaskContext::goto_restore()设置返回地址为__restore

第二种场景：当发生中断(trap)时，系统会从用户态切换到内核态处理中断，处理完成后，需要通过__restore返回到用户态继续执行

### 2.

这几行代码处理了三个关键的控制状态寄存器：
sstatus：从内核栈的trapcontext恢复到sstatus，控制处理器的特权级和中断状态
sepc：从内核栈的trapcontext恢复到sepc，存储用户程序的返回地址
sscratch：从内核栈的trapcontext恢复到sscratch，保存用户栈指针

### 3.

x2：在60行还要通过csrrw sp, sscratch, sp来交换内核栈和用户栈指针

x4：线程指针寄存器，现在用不到	# skip tp(x4), application does not use it

### 4.

sp保存内核站的地址，sscratch保存用户栈的地址

### 5.

sret指令
