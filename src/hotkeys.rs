//! OS message-based shortcuts: no keyboard hooks and no polling loop.
use std::sync::mpsc::{self, Receiver};

pub enum Action {
    Toggle,
    Lock,
}
pub struct Hotkeys {
    pub events: Receiver<Action>,
    pub available: bool,
    #[cfg(windows)]
    thread_id: u32,
    #[cfg(windows)]
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Hotkeys {
    pub fn new(ctx: eframe::egui::Context) -> Self {
        let (send, events) = mpsc::channel();
        #[cfg(windows)]
        {
            use windows_sys::Win32::{
                System::Threading::GetCurrentThreadId,
                UI::{
                    Input::KeyboardAndMouse::{
                        MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT, RegisterHotKey, UnregisterHotKey,
                    },
                    WindowsAndMessaging::{GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, WM_HOTKEY},
                },
            };
            let (ready, status) = mpsc::channel();
            let thread = std::thread::spawn(move || {
                // SAFETY: this thread owns its message queue and hotkey registrations.
                // All pointers reference initialized local storage or null (thread messages).
                unsafe {
                    let mut msg: MSG = std::mem::zeroed();
                    PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                    let flags = MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT;
                    let toggle =
                        RegisterHotKey(std::ptr::null_mut(), 1, flags, u32::from(b'O')) != 0;
                    let lock = RegisterHotKey(std::ptr::null_mut(), 2, flags, u32::from(b'L')) != 0;
                    let available = toggle && lock;
                    let _ = ready.send((GetCurrentThreadId(), available));
                    if available {
                        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                            if msg.message == WM_HOTKEY {
                                let action = match msg.wParam {
                                    1 => Action::Toggle,
                                    2 => Action::Lock,
                                    _ => continue,
                                };
                                if send.send(action).is_err() {
                                    break;
                                }
                                ctx.request_repaint();
                            }
                        }
                    }
                    if toggle {
                        UnregisterHotKey(std::ptr::null_mut(), 1);
                    }
                    if lock {
                        UnregisterHotKey(std::ptr::null_mut(), 2);
                    }
                }
            });
            let (thread_id, available) = status.recv().unwrap_or((0, false));
            Self {
                events,
                available,
                thread_id,
                thread: Some(thread),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (send, ctx);
            Self {
                events,
                available: false,
            }
        }
    }
}
impl Drop for Hotkeys {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::UI::WindowsAndMessaging::{PostThreadMessageW, WM_QUIT};
            if self.available {
                // SAFETY: message goes only to the queue owned by our worker above.
                unsafe {
                    PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0);
                }
            }
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}
