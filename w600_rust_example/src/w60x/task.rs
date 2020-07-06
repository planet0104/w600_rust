use super::w60x_api as wmapi;
// use crate::wm_println;
use alloc::boxed::Box;
use alloc::string::String;
use core::any::Any;
use cstr_core::CString;
use cty::c_void;
use spin::Mutex;
use wmapi::tls_os_status;

const DEFAULT_PRIO: u8 = 38;

pub struct OSTask {
    task_queue_ptr: *mut *mut c_void,
    task_name: String,
    _prio: u8,
}

pub struct Message {
    pub value: Option<Box<dyn Any>>,
}

pub type TaskQueueID = i32;

unsafe impl Send for OSTask {}
unsafe impl Sync for OSTask {}

impl Drop for OSTask {
    fn drop(&mut self) {
        let task_queue_ptr = unsafe { Box::from_raw(self.task_queue_ptr) };
        let _ret = unsafe { wmapi::tls_os_queue_delete(*task_queue_ptr) };
        // wm_println!("[{}] tls_os_queue_delete: {:?}", self.task_name, ret);
        // wm_println!("[{}] tls_os_task_del not call", self.task_name);
        // extern "C" fn freefun(){
        //     wm_println!("tls_os_task_del freefun...");
        // }
        // let ret = unsafe{ wmapi::tls_os_task_del(self.prio, freefun)  };
        // wm_println!("tls_os_task_del {:?}", ret);
    }
}

impl OSTask {
    pub fn rev_msg(&self) -> Option<Box<dyn Any>> {
        let mut msg_ptr: *mut c_void = core::ptr::null_mut();
        let ret = unsafe {
            wmapi::tls_os_queue_receive(
                *self.task_queue_ptr,
                &mut msg_ptr as *mut *mut c_void,
                0,
                0,
            )
        };
        if ret == tls_os_status::TLS_OS_SUCCESS {
            let mut msg: Box<Message> = unsafe { Box::from_raw(msg_ptr as *mut _) };
            msg.value.take()
        } else {
            None
        }
    }

    pub fn rev_msg_wait(&self, wait_time: u32) -> Option<Box<dyn Any>> {
        let mut msg_ptr: *mut c_void = core::ptr::null_mut();
        let ret = unsafe {
            wmapi::tls_os_queue_receive(
                *self.task_queue_ptr,
                &mut msg_ptr as *mut *mut c_void,
                0,
                wait_time,
            )
        };
        if ret == tls_os_status::TLS_OS_SUCCESS {
            let mut msg: Box<Message> = unsafe { Box::from_raw(msg_ptr as *mut _) };
            msg.value.take()
        } else {
            None
        }
    }

    pub fn get_name(&self) -> &str {
        &self.task_name
    }

    #[allow(dead_code)]
    pub fn send_msg<T: Any>(&self, value: T) {
        let msg = Box::new(Message {
            value: Some(Box::new(value)),
        });
        unsafe {
            wmapi::tls_os_queue_send(
                *self.task_queue_ptr,
                Box::into_raw(msg) as *mut i32 as *mut c_void,
                0,
            );
        }
    }

    pub fn get_queue_id(&self) -> TaskQueueID {
        unsafe { *self.task_queue_ptr as i32 }
    }
}

pub fn send_msg<T: Any>(value: T, task_queue_id: TaskQueueID) {
    let msg = Box::new(Message {
        value: Some(Box::new(value)),
    });
    unsafe {
        wmapi::tls_os_queue_send(
            task_queue_id as *mut c_void,
            Box::into_raw(msg) as *mut i32 as *mut c_void,
            0,
        );
    }
}

type TaskEntry = extern "C" fn(*mut c_void);
struct EntryParams<F>
where
    F: FnMut(OSTask),
{
    callback: F,
    task: Option<OSTask>,
}

extern "C" fn trampoline<F>(params: *mut c_void)
where
    F: FnMut(OSTask),
{
    let mut params: Box<EntryParams<F>> = unsafe { Box::from_raw(params as *mut EntryParams<F>) };
    (params.callback)(params.task.take().unwrap());
}

pub fn get_trampoline<F>(_closure: &F) -> TaskEntry
where
    F: FnMut(OSTask),
{
    trampoline::<F>
}

pub fn create<F>(name: &'static str, stk: &Mutex<[u32]>, callback: F) -> Result<(), String>
where
    F: FnMut(OSTask),
{
    let task_queue_ptr: Box<*mut c_void> = Box::new(core::ptr::null_mut());
    let task_queue_ptr_ptr = Box::into_raw(task_queue_ptr);
    let task_queue_ptr_ptr_clone = task_queue_ptr_ptr as i32;

    let prio = DEFAULT_PRIO;

    let os_task = OSTask {
        task_name: String::from(name),
        _prio: prio,
        task_queue_ptr: task_queue_ptr_ptr,
    };

    let params = Box::new(EntryParams {
        callback,
        task: Some(os_task),
    });

    let entry = get_trampoline(&params.callback);

    let param_ptr = Box::into_raw(params);

    let mut stk = stk.lock();

    let ret = unsafe {
        wmapi::tls_os_task_create(
            core::ptr::null_mut(),
            CString::new(name).unwrap().as_ptr(),
            entry,
            param_ptr as *mut _ as *mut c_void,
            &mut *stk as *mut _ as *mut u8, /* task's stack start address */
            (stk.len() * core::mem::size_of::<u32>()) as u32, /* task's stack size, unit:byte */
            prio as u32,
            0,
        )
    };
    if ret != tls_os_status::TLS_OS_SUCCESS {
        return Err(format!("{:?}", ret));
    }

    let ret =
        unsafe { wmapi::tls_os_queue_create(task_queue_ptr_ptr_clone as *mut *mut c_void, 4) };
    if ret != tls_os_status::TLS_OS_SUCCESS {
        return Err(format!("{:?}", ret));
    }

    Ok(())
}
