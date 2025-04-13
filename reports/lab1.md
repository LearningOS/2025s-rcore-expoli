# ch3 编程作业
## 获取任务信息

在 ch3 中，我们的系统已经能够支持多个任务分时轮流运行，我们希望引入一个新的系统调用 ``sys_trace``（ID 为 410）用来追踪当前任务系统调用的历史信息，并做对应的修改。定义如下。

```Rust
fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize
```
- 调用规范：
  - 这个系统调用有三种功能，根据 trace_request 的值不同，执行不同的操作：
  - 如果 trace_request 为 0，则 id 应被视作 *const u8 ，表示读取当前任务 id 地址处一个字节的无符号整数值。此时应忽略 data 参数。返回值为 id 地址处的值。
  - 如果 trace_request 为 1，则 id 应被视作 *const u8 ，表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0。
  - 如果 trace_request 为 2，表示查询当前任务调用编号为 id 的系统调用的次数，返回值为这个调用次数。本次调用也计入统计 。
  - 否则，忽略其他参数，返回值为 -1。

- 说明：
  - 你可能会注意到，这个调用的读写并不安全，使用不当可能导致崩溃。这是因为在下一章节实现地址空间之前，系统中缺乏隔离机制。所以我们 不要求你实现安全检查机制，只需通过测试用例即可 。
  - 你还可能注意到，这个系统调用读写本任务内存的功能并不是很有用。这是因为作业的灵感来源 syscall 主要依靠 trace 功能追踪其他任务的信息，但在本章节我们还没有进程、线程等概念，所以简化了操作，只要求追踪自身的信息。

## 解决思路

### 问题疑惑与思考
1. 在阅读对应的实验要求的时候，我看到了所有的任务上都有 `当前任务` 这个显眼的字段， 前两个在经过一段思考之后，发现在当前的操作系统中，并没有实现对应的虚拟文件系统，根据后面的提示，应该就是把对应的 id 转换为指针然后进行读写操作。于是可以先写出前两种情况：
```Rust
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    match _trace_request {
        0 => {
            let ptr = _id as *const u8;
            let value = unsafe{ ptr.read() };
            value as isize
        },
        1 => {
            let ptr = _id as *const u8;
            let data_ptr = _data as *mut u8;
            unsafe { *data_ptr = *ptr };
            0
        },
        2 => {
            // 待实现
        },
        _ => -1,
    }
}
```
2. 对于 _trace_request == 2 的这种情况就需要仔细考虑了
   1. 因为这个是**当前的任务**对 syscalls 的调用，所以首先需要做到的就是获取到当前任务的信息：这个可以在 TASK_MANAGER 中进行获取和操作。
   2. 在进行查询的时候，查询的也是当前任务调用编号为 id 的系统调用的次数。那么就需要保存对应的编号 id 的调用次数，那就涉及到使用什么样的数据结构进行处理。
      1. 最容易的就是使用一个全局的二维数组，我看了一下 trace 的系统调用的调用号为 400 多，如果使用二维数组的话，会造成大量的空间浪费，所以这个方案不太合理。
      2. 修改 TaskControlBlock 在，在每一个任务中开辟一个追踪数组，数组大小为 412，比 trace 的系统调用号稍微高一点。而且这个数组是随着任务量进行初始化和释放的，与任务量有关，与直接的二维数组相比，空间占用量小得多。于是采用方案2.
3. 统计点，因为是统计所有的系统调用信息与次数，那么就应该在入口处进行统计，这样的话就不会发生遗漏现象，同时入口也非常统一，便于管理。

### trace 实现

1. 修改 TaskControlBlock ，添加 trace 数组
```Rust
/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// The task syscall time trace
    pub task_trace: [u8; 412],
}
```

2. 修改初始化函数

```Rust
lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
            task_trace: [0; 412],
        }; MAX_APP_NUM];
        ...
    }
}
```

3. 添加 TaskManager 内部的操作方法（get_current_task_id）是多余的实现，没有使用到
```Rust
impl TaskManager {
    /// get current_task_id
    fn get_current_task_id(&self) -> usize {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        drop(inner);
        return current;
    }
    /// current_task trace call 
    fn get_current_task_call_syscall_id_time(&self, id: usize) -> isize {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let call_time = inner.tasks[current].task_trace[id];
        drop(inner);
        return call_time as isize;
    }
    /// increase current_task trace call
    fn increase_current_task_call_syscall_id_time(&self, id: usize) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].task_trace[id] += 1;
        drop(inner);
    }
}
```

4. 添加外部接口

```Rust
/// get current_task_id
pub fn get_current_task_id() -> usize {
    TASK_MANAGER.get_current_task_id()
}
/// get current_task trace call
pub fn get_current_task_call_syscall_id_time(id: usize) -> isize {
    TASK_MANAGER.get_current_task_call_syscall_id_time(id)
}
/// increase current_task trace call
pub fn increase_current_task_call_syscall_id_time(id: usize) {
    TASK_MANAGER.increase_current_task_call_syscall_id_time(id)
}
```

5. 添加统计点

```Rust
use crate::task::increase_current_task_call_syscall_id_time;

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    increase_current_task_call_syscall_id_time(syscall_id);
    match syscall_id {
        SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
        SYSCALL_EXIT => sys_exit(args[0] as i32),
        SYSCALL_YIELD => sys_yield(),
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TRACE => sys_trace(args[0], args[1], args[2]),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
```

6. 实现 sys_trace 系统调用

```Rust
// TODO: implement the syscall
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    match _trace_request {
        0 => {
            let ptr = _id as *const u8;
            let value = unsafe{ ptr.read() };
            value as isize
        },
        1 => {
            //   - 如果 trace_request 为 1，则 id 应被视作 *mut u8 ，
            // 表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0。
            let ptr = _id as *mut u8;
            unsafe { *ptr = _data as u8 };
            0
        },
        2 => {
            get_current_task_call_syscall_id_time(_id)
        },
        _ => -1,
    }
}
```

# 遇到的问题
## 如何在本地进行测试

一开始不知道如何在本地进行测试，于是按照 CI 上的用法，把对应的 user 与 ci_user 都 clone 下来进行测试，也达到了对应的目的。

后来经过群友的提示发现可以使用下面的方法来进行本地测试调试

```bash
make run BASE=3 LOG=Debug
``` 

## GitHub CI 失败的问题

通过查看发现是 cargo 同步 index 的时候会出现超时的现象，因为设置了 rsproxycn 的镜像，GitHub Actions 是直接跑在外网的，不需要设置代理，这个也是引起网络波动的原因，可以通过下面的方法，取消环境变量的设置，来提高 ci 构建的稳定性。

```yaml
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: |
          unset RUSTUP_DIST_SERVER
          unset RUSTUP_UPDATE_ROOT
          unset CARGO_HTTP_MULTIPLEXING
          qemu-system-riscv64 --version
          rustup target add riscv64gc-unknown-none-elf
          git config --global --add safe.directory /__w/${{ github.event.repository.name }}/${{ github.event.repository.name }}
          git clone https://github.com/LearningOS/rCore-Tutorial-Checker-2025S.git ci-user
          git clone https://github.com/LearningOS/rCore-Tutorial-Test-2025S.git ci-user/user
          ID=`git rev-parse --abbrev-ref HEAD | grep -oP 'ch\K[0-9]'`
          # cd ci-user && make test CHAPTER=$ID passwd=${{ secrets.BASE_TEST_TOKEN }}
          cd ci-user && make test CHAPTER=$ID passwd=${{ secrets.BASE_TEST_TOKEN }} > ../output.txt
          cat ../output.txt
```

## syscall_trace 1 的情况

- 下面这句话有问题，应该是 视作 *mut u8
  - 如果 trace_request 为 1，则 id 应被视作 *const u8 ，表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0。
