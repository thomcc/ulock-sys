//! Bindings for macOS's not-entirely-public ulock API, which provides
//! futex-like functionality.
//!
//! In general, caveat emptor. It's recommended that you read both the
//! [source][sys_ulock_c] and the [header][ulock_h]. In particular, I've not
//! documented the code very much beyond what's in the headers, aside from
//! supported version indications.
//!
//! # Support
//!
//! This API is available on darwin 16+ (macOS 10.12+, iOS 10.0+, tvOS 10.0+,
//! watchOS 3.0+, catalyst 13.0+), although some parts are darwin 19+ (macOS
//! 10.15+, iOS 13.0+, TODO: find out the rest). The parts on darwin 19+ are in
//! the `darwin19` module (it's just some additional constants, so not worth
//! feature gating).
//!
//! That is to say, this is pretty well supported, over 95% of users on both
//! macOS and iOS will have it (according to statscounter as of Jan 2021).
//!
//! That said, they're not public, and who knows the future, maybe Apple will
//! remove them. But probably not — they're also used by libc++, and so if they
//! ever went away, code that statically links libc++ would suddenly break,
//! but... who knows, maybe apple will do it.
//!
//! ## "weak" linking, cargo features
//!
//! As a result of that (and so that you can support older versions), if you
//! enable the `experimental-weak` feature, we'll expose a module `weak` which
//! emulates weak linking and accesses the function via `dlsym`.
//!
//! Note that on aarch64 darwin (e.g. "apple silicon", the ARM64 macbooks and
//! such) by default we don't `dlsym` even if the "experimental-weakweak"
//! feature is on unless "weak-aarch64-macos" is also specified. This is because
//! all OSes on these machines support the API. This can be overridden by
//! enabling the `weak-aarch64-macos` feature.
//!
//! [sys_ulock_c]: https://opensource.apple.com/source/xnu/xnu-6153.11.26/bsd/kern/sys_ulock.c.auto.html
//! [ulock_h]: https://opensource.apple.com/source/xnu/xnu-6153.11.26/bsd/sys/ulock.h.auto.html
//!
//! Note that the API of this crate (except for `const`s) is behind `cfg!(target_vendor = "apple")`.

#![no_std]
use cty::{c_int, c_void};

#[cfg(all(feature = "experimental-weak", target_vendor = "apple"))]
pub mod weak;

#[cfg(target_vendor = "apple")]
#[link(name = "System", kind = "dylib")]
extern "C" {
    pub fn __ulock_wait(op: u32, addr: *mut c_void, val: u64, micros: u32) -> c_int;
    pub fn __ulock_wake(op: u32, addr: *mut c_void, val: u64) -> c_int;
}

/// Operation code (these are in bits 0-8)
pub const UL_COMPARE_AND_WAIT: u32 = 1;
/// Operation code (these are in bits 0-8)
pub const UL_UNFAIR_LOCK: u32 = 2;

/// Obsolete name for [`UL_COMPARE_AND_WAIT`]. Deprecated, but provided for porting.
#[deprecated = "Use `UL_COMPARE_AND_WAIT`"]
pub const UL_OSSPINLOCK: u32 = UL_COMPARE_AND_WAIT;
/// Obsolete name for [`UL_UNFAIR_LOCK`]. Deprecated, but provided for porting.
#[deprecated = "Use `UL_UNFAIR_LOCK`"]
pub const UL_HANDOFFLOCK: u32 = UL_UNFAIR_LOCK;

/// The portion of the API only is supported on Darwin 19 and up (macOS 10.15+,
/// iOS 13.0+).
pub mod darwin19 {
    /// Operation code (these are in bits 0-8). Requires Darwin 19+.
    pub const UL_COMPARE_AND_WAIT_SHARED: u32 = 3;
    /// Operation code (these are in bits 0-8). Requires Darwin 19+.
    pub const UL_UNFAIR_LOCK64_SHARED: u32 = 4;
    /// Operation code (these are in bits 0-8). Requires Darwin 19+.
    pub const UL_COMPARE_AND_WAIT64: u32 = 5;
    /// Operation code (these are in bits 0-8). Requires Darwin 19+.
    pub const UL_COMPARE_AND_WAIT64_SHARED: u32 = 6;
    /// Flag for [`__ulock_wait`](super::__ulock_wait) (these are in bits 16-24).
    ///
    /// Use adaptive spinning when the thread that currently holds the unfair
    /// lock is on core.
    pub const ULF_WAIT_ADAPTIVE_SPIN: u32 = 0x00040000;
}

/// Flag for [`__ulock_wake`] (these are in bits 8-16)
pub const ULF_WAKE_ALL: u32 = 0x00000100;
/// Flag for [`__ulock_wake`] (these are in bits 8-16)
pub const ULF_WAKE_THREAD: u32 = 0x00000200;

/// Flag for [`__ulock_wait`] (these are in bits 16-24).
///
/// The waiter is contending on this lock for synchronization around global
/// data. This causes the workqueue subsystem to not create new threads to
/// offset for waiters on this lock.
pub const ULF_WAIT_WORKQ_DATA_CONTENTION: u32 = 0x00010000;

/// Flag only for [`__ulock_wait`] (these are in bits 16-24).
///
/// This wait is a cancelation point.
pub const ULF_WAIT_CANCEL_POINT: u32 = 0x00020000;

/// Generic flag usable with [`__ulock_wake`] or [`__ulock_wait`] (these are in bits 24-32)
pub const ULF_NO_ERRNO: u32 = 0x01000000;

// /// Mask for the `__ulock_{wait,wake}` operation code (e.g. `UL_*` constant).
// ///
// /// You probably don't need to use this constant.
// pub const UL_OPCODE_MASK: u32 = 0x000000FF;
// /// Mask for all flags. Equivalent to `!UL_OPCODE_MASK`.
// ///
// /// You probably don't need to use this constant.
// pub const UL_FLAGS_MASK: u32 = 0xFFFFFF00;
// /// In theory, a mask for the generic flags. In practice, this also contains the
// /// flags for [`__ulock_wait`]. It's unclear if this is intentional.
// ///
// /// You probably don't need to use this constant.
// pub const ULF_GENERIC_MASK: u32 = 0xFFFF0000;

// Not provided because they're different on darwin19+ from before.
//
// pub const ULF_WAIT_MASK: u32 =
//     ULF_NO_ERRNO | ULF_WAIT_WORKQ_DATA_CONTENTION | ULF_WAIT_CANCEL_POINT;
// pub const ULF_WAKE_MASK: u32 = ULF_WAKE_ALL | ULF_WAKE_THREAD | ULF_NO_ERRNO;

/// Alias for the function pointer of [`__ulock_wait`].
pub type ULockWaitFn = unsafe extern "C" fn(u32, *mut c_void, u64, u32) -> c_int;

/// Alias for the function pointer of [`__ulock_wake`].
pub type ULockWakeFn = unsafe extern "C" fn(u32, *mut c_void, u64) -> c_int;
