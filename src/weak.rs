//! `feature = "experimental-weak"` Weak linking emulation. See crate docs for info.
use super::*;

/// A dynamically loaded pair of function pointers implementing the API.
///
/// This can be returned by `ULockApi::get()`. Note that unless you can cache
/// the API reference somewhere easily, it's a bit more efficient to call
/// [`weak::ulock_wait`](ulock_wait) and/or [`weak::ulock_wake`](ulock_wake)
/// directly.
#[derive(Clone, Copy, PartialEq)]
#[repr(C)]
pub struct ULockApi {
    /// Function pointer to [`__ulock_wait`].
    pub ulock_wait: ULockWaitFn,
    /// Function pointer to [`__ulock_wake`].
    pub ulock_wake: ULockWakeFn,
}

#[cfg(all(
    target_os = "macos",
    target_arch = "aarch64",
    not(feature = "weak-aarch64-macos")
))]
mod imp {
    use super::*;
    pub(super) static API: ULockApi = ULockApi {
        ulock_wait: crate::__ulock_wait,
        ulock_wake: crate::__ulock_wake,
    };

    #[inline]
    pub(super) unsafe fn ulock_wait(
        op: u32,
        addr: *mut c_void,
        val: u64,
        micros: u32,
    ) -> Result<c_int, ApiUnsupported> {
        Ok(crate::__ulock_wait(op, addr, val, micros))
    }

    #[inline]
    pub(super) unsafe fn ulock_wake(
        op: u32,
        addr: *mut c_void,
        val: u64,
    ) -> Result<c_int, ApiUnsupported> {
        Ok(crate::__ulock_wake(op, addr, val))
    }

    #[inline]
    pub(super) fn get() -> Result<&'static ULockApi, ApiUnsupported> {
        Ok(&API)
    }
}

#[cfg(not(all(
    target_os = "macos",
    target_arch = "aarch64",
    not(feature = "weak-aarch64-macos")
)))]
mod imp {
    use super::*;
    use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering::*};

    #[repr(C)]
    struct MaybeULockApi {
        ulock_wait: AtomicUsize,
        ulock_wake: AtomicUsize,
    }

    const NOT_INIT: u8 = 0;
    const BAD_INIT: u8 = 1;
    const GOOD_INIT: u8 = 2;
    static INIT_STATE: AtomicU8 = AtomicU8::new(NOT_INIT);

    static API: MaybeULockApi = MaybeULockApi {
        ulock_wait: AtomicUsize::new(0),
        ulock_wake: AtomicUsize::new(0),
    };

    #[inline]
    pub(super) unsafe fn ulock_wait(
        op: u32,
        addr: *mut c_void,
        val: u64,
        micros: u32,
    ) -> Result<cty::c_int, ApiUnsupported> {
        let func: Option<ULockWaitFn> = core::mem::transmute(API.ulock_wait.load(Relaxed));
        if let Some(func) = func {
            return Ok((func)(op, addr, val, micros));
        }
        ulock_wait_outline(op, addr, val, micros)
    }

    #[cold]
    unsafe fn ulock_wait_outline(
        op: u32,
        addr: *mut c_void,
        val: u64,
        micros: u32,
    ) -> Result<cty::c_int, ApiUnsupported> {
        maybe_init().and_then(|_| {
            let func: ULockWaitFn = core::mem::transmute(API.ulock_wait.load(Relaxed));
            Ok(func(op, addr, val, micros))
        })
    }

    #[inline]
    pub(super) unsafe fn ulock_wake(
        op: u32,
        addr: *mut c_void,
        val: u64,
    ) -> Result<cty::c_int, ApiUnsupported> {
        let func: Option<ULockWakeFn> = core::mem::transmute(API.ulock_wake.load(Relaxed));
        if let Some(func) = func {
            return Ok((func)(op, addr, val));
        }
        ulock_wake_outline(op, addr, val)
    }

    #[cold]
    unsafe fn ulock_wake_outline(
        op: u32,
        addr: *mut c_void,
        val: u64,
    ) -> Result<cty::c_int, ApiUnsupported> {
        maybe_init().and_then(|_| {
            let func: ULockWakeFn = core::mem::transmute(API.ulock_wake.load(Relaxed));
            Ok(func(op, addr, val))
        })
    }

    #[inline]
    pub(super) fn get() -> Result<&'static super::ULockApi, ApiUnsupported> {
        maybe_init()?;
        // Note: we know that `API` will never be written to again.
        Ok(unsafe { &*(&API as *const _ as *const super::ULockApi) })
    }

    #[inline]
    fn maybe_init() -> Result<(), ApiUnsupported> {
        match INIT_STATE.load(Acquire) {
            BAD_INIT => return Err(ApiUnsupported(())),
            GOOD_INIT => {}
            NOT_INIT => init()?,
            v => {
                debug_assert!(false, "unknown state (please report this bug): {}", v);
                unsafe { core::hint::unreachable_unchecked() };
            }
        }
        debug_assert_eq!(INIT_STATE.load(Acquire), GOOD_INIT);
        debug_assert!(API.ulock_wait.load(Relaxed) != 0);
        debug_assert!(API.ulock_wake.load(Relaxed) != 0);
        Ok(())
    }

    fn init() -> Result<(), ApiUnsupported> {
        let ulock_wait = unsafe { load_sym("__ulock_wait\0") as usize };
        let ulock_wake = unsafe { load_sym("__ulock_wake\0") as usize };
        if ulock_wait == 0 || ulock_wake == 0 {
            let old = match INIT_STATE.compare_exchange(NOT_INIT, BAD_INIT, AcqRel, Acquire) {
                Ok(v) => v,
                Err(v) => v,
            };
            return match old {
                NOT_INIT => Err(ApiUnsupported(())),
                GOOD_INIT => Ok(()),
                BAD_INIT => Err(ApiUnsupported(())),
                v => {
                    debug_assert!(false, "unknown state (please report this bug): {}", v);
                    unsafe { core::hint::unreachable_unchecked() };
                }
            };
        }

        let old = API
            .ulock_wait
            .compare_exchange(0, ulock_wait, Relaxed, Relaxed);
        debug_assert!(old.is_ok() || old == Err(ulock_wait));

        let old = API
            .ulock_wake
            .compare_exchange(0, ulock_wake, Relaxed, Relaxed);
        debug_assert!(old.is_ok() || old == Err(ulock_wake));

        if cfg!(any(target_arch = "x86", target_arch = "x86_64")) {
            // Force bus lock w/ `lock xchg` as an optimization. Note that we're
            // still sound if this doesn't work, but it can help ensure that
            // this value is seen by other threads sooner.
            let old = INIT_STATE.swap(GOOD_INIT, SeqCst);
            debug_assert_ne!(old, BAD_INIT);
        } else {
            INIT_STATE.store(GOOD_INIT, Release);
        }
        Ok(())
    }

    #[inline]
    unsafe fn load_sym(s: &str) -> *mut c_void {
        const RTLD_DEFAULT: *mut c_void = -2isize as *mut c_void;
        extern "C" {
            fn dlsym(h: *mut c_void, s: *const cty::c_char) -> *mut c_void;
        }
        debug_assert_eq!(s.as_bytes()[s.len() - 1], b'\0');
        dlsym(RTLD_DEFAULT, s.as_ptr().cast())
    }

    // Try and run the initalization before main. Not required for
    // correctness/soundness, just helps us always use the fast path.
    #[used]
    #[cfg(link_section = "__DATA,__mod_init_func")]
    static CTOR: extern "C" fn() = init_function;
    #[allow(dead_code)]
    extern "C" fn init_function() {
        drop(init());
    }
}

/// Equivalent to [`__ulock_wait`], but lazy-loads and returns
/// `Err(ApiUnsupported)` if this machine doesn't support the API.
///
/// It's generally faster to call this than to call
/// `ULockApi::get()?.ulock_wait(...)` unless you can cache the result of `get`
/// somewhere easy.
///
/// # Safety
/// Same as [`__ulock_wait`].
#[inline]
pub unsafe fn ulock_wait(
    op: u32,
    addr: *mut c_void,
    val: u64,
    micros: u32,
) -> Result<cty::c_int, ApiUnsupported> {
    imp::ulock_wait(op, addr, val, micros)
}

/// Equivalent to [`__ulock_wake`], but lazy-loads and returns
/// `Err(ApiUnsupported)` if this machine doesn't support the API.
///
/// It's generally faster to call this than to call
/// `ULockApi::get()?.ulock_wake(...)` unless you can cache the result of `get`
/// somewhere easy.
///
/// # Safety
/// Same as [`__ulock_wake`]
#[inline]
pub unsafe fn ulock_wake(
    op: u32,
    addr: *mut c_void,
    val: u64,
) -> Result<cty::c_int, ApiUnsupported> {
    imp::ulock_wake(op, addr, val)
}

impl ULockApi {
    /// Get a reference to the API, if supported, loading it if not already
    /// loaded. This function is relatively fast for non-first time loads (which
    /// should be rare), but it's still likely (very slightly) better to call
    /// [`ulock_wait`] or [`ulock_wake`] unless you cache it somewhere quite
    /// cheap to access.
    #[inline]
    pub fn get() -> Result<&'static Self, ApiUnsupported> {
        imp::get()
    }
}

/// Error indicating that we failed to load the API.
#[derive(Clone, PartialEq, Copy)]
pub struct ApiUnsupported(());

impl core::fmt::Display for ApiUnsupported {
    #[cold]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ulock api unsupported")
    }
}

impl core::fmt::Debug for ApiUnsupported {
    #[cold]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("ulock api unsupported")
    }
}
