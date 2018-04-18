use std::io;
use std::sync::{Arc, Weak, RwLock};
use std::sync::atomic::{self, AtomicBool};
use std::thread::{self, JoinHandle};


#[derive(PartialEq, Eq, Debug)]
pub enum ThreadStatus {
    Running,
    Terminated,
}

pub struct ThreadHandle<T> {
    interrupted: Weak<AtomicBool>,
    join_handle: RwLock<Option<JoinHandle<T>>>,
}

impl<T> ThreadHandle<T> where T: Send + 'static {
    pub fn spawn<F>(name: String, runnable: F) -> io::Result<Self> where
        F: FnOnce(Arc<AtomicBool>) -> T, F: Send + 'static
    {
        let interrupted_flag = Arc::new(AtomicBool::new(false));
        let interrupted = Arc::downgrade(&interrupted_flag);
        let join_handle = thread::Builder::new()
            .name(name)
            .spawn(move || {
                runnable(interrupted_flag)
            })?;
        Ok(ThreadHandle {
            interrupted: interrupted,
            join_handle: RwLock::new(Some(join_handle)),
        })
    }

    pub fn status(&self) -> ThreadStatus {
        if self.interrupted.upgrade().is_some() {
            ThreadStatus::Running
        } else {
            ThreadStatus::Terminated
        }
    }

    pub fn interrupt(&self) -> Result<bool, ()> {
        if let Some(interrupted_flag) = self.interrupted.upgrade() {
            let previous = interrupted_flag.compare_and_swap(false, true, atomic::Ordering::Relaxed);
            Ok(previous)
        } else {
            Err(())
        }
    }

    pub fn join(&self) -> Option<thread::Result<T>> {
        if self.join_handle.read().unwrap().is_some() {
            if let Some(join_handle) = self.join_handle.write().unwrap().take() {
                Some(join_handle.join())
            } else {
                None
            }
        } else {
            None
        }
    }
}


#[cfg(test)]
mod tests {
    use std::thread::sleep;
    use std::time::Duration;
    use super::*;

    #[test]
    fn test_status() {
        let handle = ThreadHandle::spawn("Test status".to_string(), move |_| {
            sleep(Duration::from_millis(1000));
        }).unwrap();
        sleep(Duration::from_millis(700));
        assert_eq!(ThreadStatus::Running, handle.status());
        sleep(Duration::from_millis(700));
        assert_eq!(ThreadStatus::Terminated, handle.status());
    }

    #[test]
    fn test_join_ok() {
        let handle = ThreadHandle::spawn("Test join ok".to_string(), move |_| {
            sleep(Duration::from_millis(1000));
            17
        }).unwrap();
        assert_eq!(17, handle.join().unwrap().unwrap());
        assert!(handle.join().is_none());
    }

    #[test]
    fn test_join_error() {
        let handle = ThreadHandle::spawn("Test join error".to_string(), move |_| {
            sleep(Duration::from_millis(1000));
            panic!("");
        }).unwrap();
        assert!(handle.join().unwrap().is_err());
        assert!(handle.join().is_none());
    }

    #[test]
    fn test_interrupt() {
        let handle = ThreadHandle::spawn("Test interrupt".to_string(), move |interrupted| {
            let mut i = 0;
            while !interrupted.load(atomic::Ordering::Relaxed) {
                sleep(Duration::from_millis(200));
                i = i + 1;
            }
            i
        }).unwrap();
        sleep(Duration::from_millis(1000));
        assert_eq!(false, handle.interrupt().unwrap());
        assert_eq!(true, handle.interrupt().unwrap());
        let result = handle.join().unwrap().unwrap();
        assert!(result > 0 && result < 10);
        assert!(handle.interrupt().is_err());
    }
}
