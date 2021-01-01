# `ulock-sys`

Rust bindings for Darwin's (e.g. macOS's/iOS's) not-entirely-public ulock API, which provides futex-like functionality.

In general, caveat emptor. It's recommended that you read both the [source](https://opensource.apple.com/source/xnu/xnu-6153.11.26/bsd/kern/sys_ulock.c.auto.html) and the [header](https://opensource.apple.com/source/xnu/xnu-6153.11.26/bsd/sys/ulock.h.auto.html), and possibly skim libdispatch's or libc++'s usage.

In particular, docs beyond what's in the headers are sparse at best.

## Support

This API is available on darwin 16+ (macOS 10.12+, iOS 10.0+, tvOS 10.0+, watchOS 3.0+, catalyst 13.0+), although some parts are darwin 19+ (macOS 10.15+, iOS 13.0+, TODO: find out the rest). The parts on darwin 19+ are in the `darwin19` module (it's just some additional constants, so not worth feature gating).

That is to say, this is pretty well supported, over 95% of users on both macOS and iOS will have it (according to statscounter as of Jan 2021).

That said, they're not public, and who knows the future, maybe Apple will remove them. But probably not — they're also used by libc++, and so if they ever went away, code that statically links libc++ would suddenly break, but... who knows, maybe apple will do it.

### "weak" linking, cargo features

As a result of that (and so that you can support older versions), if you enable the `experimental-weak` feature, we'll expose a module `weak` which emulates weak linking and accesses the function via `dlsym`.

Note that on aarch64 darwin (e.g. "apple silicon", the ARM64 macbooks and such) by default we don't `dlsym` even if the "experimental-weak" feature is on unless "weak-aarch64-macos" is also specified. This is because all OSes on these machines support the API. This can be overridden by enabling the `weak-aarch64-macos` feature.
