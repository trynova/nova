// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! See the big comment in jit/AtomicOperations.h for an explanation.

use core::ptr::NonNull;

// is_64bit = "JS_64BIT" in buildconfig.defines
// cpu_arch = buildconfig.substs["TARGET_CPU"]
// is_gcc = buildconfig.substs["CC_TYPE"] == "gcc"

macro_rules! fence {
    (true, x86) => {
        "mfence"
    };
    (true, aarch64) => {
        "dmb ish"
    };
    (true, arm) => {
        "dmb sy"
    };
    (false, $_: tt) => {
        ""
    };
}

macro_rules! gen_load {
    (u8, $ptr: ident, $barrier: tt) => {
        let z: u8;
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "mov {val}, [{ptr}]",
                fence!(false, x86),
                ptr = in(reg) ptr,
                val = lateout(reg_byte) z,
                options(preserves_flags, nostack, pure, readonly)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!(
                "ldrb {val:w}, [{ptr}]",
                fence!($barrier, aarch64),
                ptr = in(reg) ptr,
                val = lateout(reg) z,
                options(preserves_flags, nostack, pure, readonly)
            );
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            core::arch::asm!(
                "ldrb {val:w}, [{ptr}]",
                fence!($barrier, arm),
                ptr = in(reg) ptr,
                val = lateout(reg) z,
                options(preserves_flags, nostack, pure, readonly)
            );
        }

        return z;
    };
    (u16, $ptr: ident, $barrier: tt) => {
        let z: u16;
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "mov {val:x}, [{ptr}]",
                fence!(false, x86),
                ptr = in(reg) ptr,
                val = lateout(reg) z,
                options(preserves_flags, nostack, pure, readonly)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!(
                "ldrh {val:w}, [{ptr}]",
                fence!($barrier, aarch64),
                ptr = in(reg) ptr,
                val = lateout(reg) z,
                options(preserves_flags, nostack, pure, readonly)
            );
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            core::arch::asm!(
                "ldrh {val:w}, [{ptr}]",
                fence!($barrier, arm),
                ptr = in(reg) ptr,
                val = lateout(reg) z,
                options(preserves_flags, nostack, pure, readonly)
            );
        }

        return z;
    };
    (u32, $ptr: ident, $barrier: tt) => {
        let z: u32;
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "mov {val:e}, [{ptr}]",
                fence!(false, x86),
                ptr = in(reg) ptr,
                val = lateout(reg) z,
                options(preserves_flags, nostack, pure, readonly)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!(
                "ldr {val:w}, [{ptr}]",
                fence!($barrier, aarch64),
                ptr = in(reg) ptr,
                val = lateout(reg) z,
                options(preserves_flags, nostack, pure, readonly)
            );
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            core::arch::asm!(
                "ldr {val:w}, [{ptr}]",
                fence!($barrier, arm),
                ptr = in(reg) ptr,
                val = lateout(reg) z,
                options(preserves_flags, nostack, pure, readonly)
            );
        }

        return z;
    };
    (u64, $ptr: ident, $barrier: tt) => {
        let z: u64;
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!(
                "mov {val:r}, [{ptr}]",
                fence!(false, x86),
                ptr = in(reg) ptr,
                val = lateout(reg) z,
                options(preserves_flags, nostack, pure, readonly)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!(
                "ldr {val:x}, [{ptr}]",
                fence!($barrier, aarch64),
                ptr = in(reg) ptr,
                val = lateout(reg) z,
                options(preserves_flags, nostack, pure, readonly)
            );
        }

        #[cfg(any(target_arch = "x86", target_arch = "arm"))]
        unsafe {
            const { panic!("Unexpected size") }
        }

        return z;
    };
    ($type: ty, $ptr: ident, $barrier: tt) => {
        panic!("Unsupported type");
    };
}

macro_rules! gen_store {
    (u8, $ptr: ident, $val: ident, $barrier: tt) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "mov [{ptr}], {val}",
                fence!($barrier, x86),
                ptr = in(reg) ptr,
                val = in(reg_byte) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!(
                fence!($barrier, aarch64),
                "str {val:w}, [{ptr}]",
                fence!($barrier, aarch64),
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            core::arch::asm!(
                fence!($barrier, arm),
                "str {val:w}, [{ptr}]",
                fence!($barrier, arm),
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(preserves_flags, nostack)
            );
        }
    };
    (u16, $ptr: ident, $val: ident, $barrier: tt) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "mov [{ptr}], {val:x}",
                fence!($barrier, x86),
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!(
                fence!($barrier, aarch64),
                "str {val:w}, [{ptr}]",
                fence!($barrier, aarch64),
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            core::arch::asm!(
                fence!($barrier, arm),
                "str {val:w}, [{ptr}]",
                fence!($barrier, arm),
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(preserves_flags, nostack)
            );
        }
    };
    (u32, $ptr: ident, $val: ident, $barrier: tt) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "mov [{ptr}], {val:e}",
                fence!($barrier, x86),
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!(
                fence!($barrier, aarch64),
                "str {val:w}, [{ptr}]",
                fence!($barrier, aarch64),
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            core::arch::asm!(
                fence!($barrier, arm),
                "str {val:w}, [{ptr}]",
                fence!($barrier, arm),
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(preserves_flags, nostack)
            );
        }
    };
    (u64, $ptr: ident, $val: ident, $barrier: tt) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!(
                "mov [{ptr}], {val:r}",
                fence!($barrier, x86),
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!(
                fence!($barrier, aarch64),
                "str {val:x}, [{ptr}]",
                fence!($barrier, aarch64),
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(any(target_arch = "x86", target_arch = "arm"))]
        unsafe {
            const { panic!("Unexpected size") }
        }
    };
}

macro_rules! gen_exchange {
    (u8, $ptr: ident, $val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "xchg [{ptr}], {val}",
                ptr = in(reg) ptr,
                val = inout(reg_byte) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let res: u8;
            core::arch::asm!(
                "dmb ish",
                "0:",
                "ldxr {res:w}, [{ptr}]",
                "stxr {scratch:w}, {val:w}, [{ptr}]",
                "cbnz {scratch:w}, 0b",
                "dmb ish",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(nostack)
            );
            $val = res;
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            let res: u8;
            core::arch::asm!(
                "dmb sy",
                "0:",
                "ldrex {res:w}, [{ptr}]",
                "strex {scratch:w}, {val:w}, [{ptr}]",
                "cmp {scratch:w}, #1",
                "beq 0b",
                "dmb sy",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(nostack)
            );
            $val = res;
        }

        return $val;
    };
    (u16, $ptr: ident, $val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "xchg [{ptr}], {val:x}",
                ptr = in(reg) ptr,
                val = inout(reg) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let res: u16;
            core::arch::asm!(
                "dmb ish",
                "0:",
                "ldxr {res:w}, [{ptr}]",
                "stxr {scratch:w}, {val:w}, [{ptr}]",
                "cbnz {scratch:w}, 0b",
                "dmb ish",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(nostack)
            );
            $val = res;
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            let res: u8;
            core::arch::asm!(
                "dmb sy",
                "0:",
                "ldrex {res:w}, [{ptr}]",
                "strex {scratch:w}, {val:w}, [{ptr}]",
                "cmp {scratch:w}, #1",
                "beq 0b",
                "dmb sy",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(nostack)
            );
            $val = res;
        }

        return $val;
    };
    (u32, $ptr: ident, $val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "xchg [{ptr}], {val:e}",
                ptr = in(reg) ptr,
                val = inout(reg) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let res: u32;
            core::arch::asm!(
                "dmb ish",
                "0:",
                "ldxr {res:w}, [{ptr}]",
                "stxr {scratch:w}, {val:w}, [{ptr}]",
                "cbnz {scratch:w}, 0b",
                "dmb ish",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(nostack)
            );
            $val = res;
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            let res: u8;
            core::arch::asm!(
                "dmb sy",
                "0:",
                "ldrex {res:w}, [{ptr}]",
                "strex {scratch:w}, {val:w}, [{ptr}]",
                "cmp {scratch:w}, #1",
                "beq 0b",
                "dmb sy",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(nostack)
            );
            $val = res;
        }

        return $val;
    };
    (u64, $ptr: ident, $val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!(
                "xchg [{ptr}], {val:r}",
                ptr = in(reg) ptr,
                val = inout(reg) $val,
                options(preserves_flags, nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let res: u64;
            core::arch::asm!(
                "dmb ish",
                "0:",
                "ldxr {res:x}, [{ptr}]",
                "stxr {scratch:w}, {val:x}, [{ptr}]",
                "cbnz {scratch:w}, 0b",
                "dmb ish",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                val = in(reg) $val,
                options(nostack)
            );
            $val = res;
        }

        #[cfg(any(target_arch = "x86", target_arch = "arm"))]
        unsafe {
            const { panic!("Unexpected size") }
        }

        return $val;
    };
}

macro_rules! gen_cmpxchg {
    (u8, $ptr: ident, $old_val: ident, $new_val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "lock; cmpxchg [{ptr}], {new_val}",
                // Load old_val into RAX as input/output register
                inout("al") $old_val,
                ptr = in(reg) ptr,
                new_val = in(reg_byte) $new_val,
                options(nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let res: u8;
            core::arch::asm!(
                "dmb ish",
                "0:",
                "uxtb {scratch:w}, {old_val:w}",
                "ldxr {res:w} [{ptr}]",
                "cmp {res:w}, {scratch:w}",
                "b.ne 1f",
                "stxr {scratch:w}, {new_val:w}, [{ptr}]",
                "cbnz {scratch:w}, 0b",
                "1: dmb ish",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                old_val = in(reg) $old_val,
                new_val = in(reg) $new_val,
                options(nostack)
            );
            $old_val = res;
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            let res: u8;
            core::arch::asm!(
                "dmb sy",
                "0:",
                "uxtb {scratch:w}, {old_val:w}",
                "ldrex {res:w} [{ptr}]",
                "cmp {res:w}, {scratch:w}",
                "bne 1f",
                "strex {scratch:w}, {new_val:w}, [{ptr}]",
                "cmp {scratch:w}, #1",
                "beq 0b",
                "1: dmb sy",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                old_val = in(reg) $old_val,
                new_val = in(reg) $new_val,
                options(nostack)
            );
            $old_val = res;
        }

        return $old_val;
    };
    (u16, $ptr: ident, $old_val: ident, $new_val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "lock; cmpxchg [{ptr}], {new_val:x}",
                // Load old_val into RAX as input/output register
                inout("ax") $old_val,
                ptr = in(reg) ptr,
                new_val = in(reg) $new_val,
                options(nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let res: u16;
            core::arch::asm!(
                "dmb ish",
                "0:",
                "uxth {scratch:w}, {old_val:w}",
                "ldxr {res:w} [{ptr}]",
                "cmp {res:w}, {scratch:w}",
                "b.ne 1f",
                "stxr {scratch:w}, {new_val:w}, [{ptr}]",
                "cbnz {scratch:w}, 0b",
                "1: dmb ish",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                old_val = in(reg) $old_val,
                new_val = in(reg) $new_val,
                options(nostack)
            );
            $old_val = res;
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            let res: u16;
            core::arch::asm!(
                "dmb sy",
                "0:",
                "uxth {scratch:w}, {old_val:w}",
                "ldrex {res:w} [{ptr}]",
                "cmp {res:w}, {scratch:w}",
                "bne 1f",
                "strex {scratch:w}, {new_val:w}, [{ptr}]",
                "cmp {scratch:w}, #1",
                "beq 0b",
                "1: dmb sy",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                old_val = in(reg) $old_val,
                new_val = in(reg) $new_val,
                options(nostack)
            );
            $old_val = res;
        }

        return $old_val;
    };
    (u32, $ptr: ident, $old_val: ident, $new_val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            core::arch::asm!(
                "lock; cmpxchg [{ptr}], {new_val:e}",
                // Load old_val into RAX as input/output register
                inout("eax") $old_val,
                ptr = in(reg) ptr,
                new_val = in(reg) $new_val,
                options(nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let res: u32;
            core::arch::asm!(
                "dmb ish",
                "0:",
                "mov {scratch:w}, {old_val:w}",
                "ldxr {res:w} [{ptr}]",
                "cmp {res:w}, {scratch:w}",
                "b.ne 1f",
                "stxr {scratch:w}, {new_val:w}, [{ptr}]",
                "cbnz {scratch:w}, 0b",
                "1: dmb ish",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                old_val = in(reg) $old_val,
                new_val = in(reg) $new_val,
                options(nostack)
            );
            $old_val = res;
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            let res: u32;
            core::arch::asm!(
                "dmb sy",
                "0:",
                "mov {scratch:w}, {old_val:w}",
                "ldrex {res:w} [{ptr}]",
                "cmp {res:w}, {scratch:w}",
                "bne 1f",
                "strex {scratch:w}, {new_val:w}, [{ptr}]",
                "cmp {scratch:w}, #1",
                "beq 0b",
                "1: dmb sy",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                old_val = in(reg) $old_val,
                new_val = in(reg) $new_val,
                options(nostack)
            );
            $old_val = res;
        }

        return $old_val;
    };
    (u64, $ptr: ident, $old_val: ident, $new_val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(target_arch = "x86")]
        unsafe {
            let [b0, b1, b2, b3, b4, b5, b6, b7] = $old_val.to_le_bytes();
            let old_bot = u32::from_le_bytes([b0, b1, b2, b3]);
            let old_top = u32::from_le_bytes([b4, b5, b6, b7]);
            let [b0, b1, b2, b3, b4, b5, b6, b7] = $new_val.to_le_bytes();
            let new_bot = u32::from_le_bytes([b0, b1, b2, b3]);
            let new_top = u32::from_le_bytes([b4, b5, b6, b7]);
            core::arch::asm!(
                "lock; cmpxchg8b [{ptr}]",
                // Load old_val into EDX:EAX (high:low).
                inout("edx") old_top,
                inout("eax") old_bot,
                ptr = in(reg) ptr,
                // Load old_val into ECX:EBX (high:low).
                in("ecx") new_top,
                in("ebx") new_bot,
                options(nostack)
            );
            let [b0, b1, b2, b3] = old_bot.to_le_bytes();
            let [b4, b5, b6, b7] = old_top.to_le_bytes();
            $old_val = u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, b7]);
        }

        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!(
                "lock; cmpxchg [{ptr}], {new_val:r}",
                // Load old_val into RAX as input/output register
                inout("rax") $old_val,
                ptr = in(reg) ptr,
                new_val = in(reg) $new_val,
                options(nostack)
            );
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let res: u64;
            core::arch::asm!(
                "dmb ish",
                "0:",
                "mov {scratch:w}, {old_val:w}",
                "ldxr {res:w} [{ptr}]",
                "cmp {res:w}, {scratch:w}",
                "b.ne 1f",
                "stxr {scratch:w}, {new_val:w}, [{ptr}]",
                "cbnz {scratch:w}, 0b",
                "1: dmb ish",
                res = out(reg) res,
                scratch = out(reg) _,
                ptr = in(reg) ptr,
                old_val = in(reg) $old_val,
                new_val = in(reg) $new_val,
                options(nostack)
            );
            $old_val = res;
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            let [b0, b1, b2, b3, b4, b5, b6, b7] = $old_val.to_le_bytes();
            let old_bot = u32::from_le_bytes([b0, b1, b2, b3]);
            let old_top = u32::from_le_bytes([b4, b5, b6, b7]);
            let [b0, b1, b2, b3, b4, b5, b6, b7] = $new_val.to_le_bytes();
            let new_bot = u32::from_le_bytes([b0, b1, b2, b3]);
            let new_top = u32::from_le_bytes([b4, b5, b6, b7]);
            core::arch::asm!(
                "dmb sy",
                "0: ldrexd r0 r1 [{ptr}]",
                "cmp r0 {old_bot}",
                "b.ne 1f",
                "cmp r1 {old_top}",
                "b.ne 1f",
                "mov r2, {new_bot}"
                "mov r3, {new_top}"
                "strexd r4, r2, r3, [{ptr}]"
                "cmp r4, #1",
                "beq 0b",
                "1: dmb sy",
                "mov {old_bot} r0",
                "mov {old_top} r1",
                inout(reg) old_bot,
                inout(reg) old_top,
                ptr = in(reg) ptr,
                new_bot = in(reg) new_bot,
                new_top = in(reg) new_top,
                out("r0") _,
                out("r1") _,
                out("r2") _,
                out("r3") _,
                out("r4") _,
                options(nostack)
            );
            let [b0, b1, b2, b3] = old_bot.to_le_bytes();
            let [b4, b5, b6, b7] = old_top.to_le_bytes();
            $old_val = u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, b7]);
        }

        return $old_val;
    };
}

macro_rules! fetchop {
    // The `add` operation can be optimized with XADD.
    (add, x86) => {
        "lock; xadd {val}, [{ptr}]"
    };
    (or, x86) => {
        "or {val}, {scratch}"
    };
    (xor, x86) => {
        "xor {val}, {scratch}"
    };
    (add, aarch64) => {
        "add {val}, {scratch}"
    };
    (or, aarch64) => {
        "orr {val}, {scratch}"
    };
    (xor, aarch64) => {
        "eor {val}, {scratch}"
    };
    (add, arm) => {
        "add {val}, {scratch}"
    };
    (or, arm) => {
        "or {val}, {scratch}"
    };
    (xor, arm) => {
        "xor {val}, {scratch}"
    };
}

macro_rules! gen_fetchop {
    (u8, $op: tt, $ptr: ident, $val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // The `add` operation can be optimized with XADD.
            //     if op == "add":
            //         insns = ""
            //         if size == 8:
            //             insns += fmt_insn("lock; xaddb %[val], (%[addr])")
            //         elif size == 16:
            //             insns += fmt_insn("lock; xaddw %[val], (%[addr])")
            //         elif size == 32:
            //             insns += fmt_insn("lock; xaddl %[val], (%[addr])")
            //         else:
            //             assert size == 64
            //             insns += fmt_insn("lock; xaddq %[val], (%[addr])")
            //         return """
            //             INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //                 asm volatile (%(insns)s
            //                     : [val] "+&r" (val)
            //                     : [addr] "r" (addr)
            //                     : "memory", "cc");
            //                 return val;
            //             }""" % {
            //             "cpp_type": cpp_type,
            //             "fun_name": fun_name,
            //             "insns": insns,
            //         }
            //     // Use a +a constraint to ensure `res` is stored in RAX. This is required
            //     // for the CMPXCHG instruction.
            //     insns = ""
            //     if size == 8:
            //         insns += fmt_insn("movb (%[addr]), %[res]")
            //         insns += fmt_insn("0: movb %[res], %[scratch]")
            //         insns += fmt_insn("OPb %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgb %[scratch], (%[addr])")
            //     elif size == 16:
            //         insns += fmt_insn("movw (%[addr]), %[res]")
            //         insns += fmt_insn("0: movw %[res], %[scratch]")
            //         insns += fmt_insn("OPw %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgw %[scratch], (%[addr])")
            //     elif size == 32:
            //         insns += fmt_insn("movl (%[addr]), %[res]")
            //         insns += fmt_insn("0: movl %[res], %[scratch]")
            //         insns += fmt_insn("OPl %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgl %[scratch], (%[addr])")
            //     else:
            //         assert size == 64
            //         insns += fmt_insn("movq (%[addr]), %[res]")
            //         insns += fmt_insn("0: movq %[res], %[scratch]")
            //         insns += fmt_insn("OPq %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgq %[scratch], (%[addr])")
            //     insns = insns.replace("OP", op)
            //     insns += fmt_insn("jnz 0b")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res, scratch;
            //             asm volatile (%(insns)s
            //                 : [res] "=&a" (res), [scratch] "=&r" (scratch)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[cfg(target_arch = "aarch64")]
        {
            //     insns = ""
            //     insns += fmt_insn("dmb ish")
            //     insns += fmt_insn("0:")
            //     if size == 8:
            //         insns += fmt_insn("ldxrb %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxrb %w[scratch2], %w[scratch1], [%x[addr]]")
            //     elif size == 16:
            //         insns += fmt_insn("ldxrh %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxrh %w[scratch2], %w[scratch1], [%x[addr]]")
            //     elif size == 32:
            //         insns += fmt_insn("ldxr %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxr %w[scratch2], %w[scratch1], [%x[addr]]")
            //     else:
            //         assert size == 64
            //         insns += fmt_insn("ldxr %x[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxr %w[scratch2], %x[scratch1], [%x[addr]]")
            //     cpu_op = op
            //     if cpu_op == "or":
            //         cpu_op = "orr"
            //     if cpu_op == "xor":
            //         cpu_op = "eor"
            //     insns = insns.replace("OP", cpu_op)
            //     insns += fmt_insn("cbnz %w[scratch2], 0b")
            //     insns += fmt_insn("dmb ish")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res;
            //             uintptr_t scratch1, scratch2;
            //             asm volatile (%(insns)s
            //                 : [res] "=&r" (res), [scratch1] "=&r" (scratch1), [scratch2] "=&r"(scratch2)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            //     insns = ""
            //     insns += fmt_insn("dmb sy")
            //     insns += fmt_insn("0:")
            //     if size == 8:
            //         insns += fmt_insn("ldrexb %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strexb %[scratch2], %[scratch1], [%[addr]]")
            //     elif size == 16:
            //         insns += fmt_insn("ldrexh %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strexh %[scratch2], %[scratch1], [%[addr]]")
            //     else:
            //         assert size == 32
            //         insns += fmt_insn("ldrex %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strex %[scratch2], %[scratch1], [%[addr]]")
            //     cpu_op = op
            //     if cpu_op == "or":
            //         cpu_op = "orr"
            //     if cpu_op == "xor":
            //         cpu_op = "eor"
            //     insns = insns.replace("OP", cpu_op)
            //     insns += fmt_insn("cmp %[scratch2], #1")
            //     insns += fmt_insn("beq 0b")
            //     insns += fmt_insn("dmb sy")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res;
            //             uintptr_t scratch1, scratch2;
            //             asm volatile (%(insns)s
            //                 : [res] "=&r" (res), [scratch1] "=&r" (scratch1), [scratch2] "=&r"(scratch2)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[expect(unreachable_code)]
        const { panic!("Unexpected arch") }
    };
    (u16, $op: tt, $ptr: ident, $val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // The `add` operation can be optimized with XADD.
            //     if op == "add":
            //         insns = ""
            //         if size == 8:
            //             insns += fmt_insn("lock; xaddb %[val], (%[addr])")
            //         elif size == 16:
            //             insns += fmt_insn("lock; xaddw %[val], (%[addr])")
            //         elif size == 32:
            //             insns += fmt_insn("lock; xaddl %[val], (%[addr])")
            //         else:
            //             assert size == 64
            //             insns += fmt_insn("lock; xaddq %[val], (%[addr])")
            //         return """
            //             INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //                 asm volatile (%(insns)s
            //                     : [val] "+&r" (val)
            //                     : [addr] "r" (addr)
            //                     : "memory", "cc");
            //                 return val;
            //             }""" % {
            //             "cpp_type": cpp_type,
            //             "fun_name": fun_name,
            //             "insns": insns,
            //         }
            //     // Use a +a constraint to ensure `res` is stored in RAX. This is required
            //     // for the CMPXCHG instruction.
            //     insns = ""
            //     if size == 8:
            //         insns += fmt_insn("movb (%[addr]), %[res]")
            //         insns += fmt_insn("0: movb %[res], %[scratch]")
            //         insns += fmt_insn("OPb %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgb %[scratch], (%[addr])")
            //     elif size == 16:
            //         insns += fmt_insn("movw (%[addr]), %[res]")
            //         insns += fmt_insn("0: movw %[res], %[scratch]")
            //         insns += fmt_insn("OPw %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgw %[scratch], (%[addr])")
            //     elif size == 32:
            //         insns += fmt_insn("movl (%[addr]), %[res]")
            //         insns += fmt_insn("0: movl %[res], %[scratch]")
            //         insns += fmt_insn("OPl %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgl %[scratch], (%[addr])")
            //     else:
            //         assert size == 64
            //         insns += fmt_insn("movq (%[addr]), %[res]")
            //         insns += fmt_insn("0: movq %[res], %[scratch]")
            //         insns += fmt_insn("OPq %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgq %[scratch], (%[addr])")
            //     insns = insns.replace("OP", op)
            //     insns += fmt_insn("jnz 0b")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res, scratch;
            //             asm volatile (%(insns)s
            //                 : [res] "=&a" (res), [scratch] "=&r" (scratch)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[cfg(target_arch = "aarch64")]
        {
            //     insns = ""
            //     insns += fmt_insn("dmb ish")
            //     insns += fmt_insn("0:")
            //     if size == 8:
            //         insns += fmt_insn("ldxrb %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxrb %w[scratch2], %w[scratch1], [%x[addr]]")
            //     elif size == 16:
            //         insns += fmt_insn("ldxrh %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxrh %w[scratch2], %w[scratch1], [%x[addr]]")
            //     elif size == 32:
            //         insns += fmt_insn("ldxr %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxr %w[scratch2], %w[scratch1], [%x[addr]]")
            //     else:
            //         assert size == 64
            //         insns += fmt_insn("ldxr %x[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxr %w[scratch2], %x[scratch1], [%x[addr]]")
            //     cpu_op = op
            //     if cpu_op == "or":
            //         cpu_op = "orr"
            //     if cpu_op == "xor":
            //         cpu_op = "eor"
            //     insns = insns.replace("OP", cpu_op)
            //     insns += fmt_insn("cbnz %w[scratch2], 0b")
            //     insns += fmt_insn("dmb ish")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res;
            //             uintptr_t scratch1, scratch2;
            //             asm volatile (%(insns)s
            //                 : [res] "=&r" (res), [scratch1] "=&r" (scratch1), [scratch2] "=&r"(scratch2)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            //     insns = ""
            //     insns += fmt_insn("dmb sy")
            //     insns += fmt_insn("0:")
            //     if size == 8:
            //         insns += fmt_insn("ldrexb %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strexb %[scratch2], %[scratch1], [%[addr]]")
            //     elif size == 16:
            //         insns += fmt_insn("ldrexh %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strexh %[scratch2], %[scratch1], [%[addr]]")
            //     else:
            //         assert size == 32
            //         insns += fmt_insn("ldrex %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strex %[scratch2], %[scratch1], [%[addr]]")
            //     cpu_op = op
            //     if cpu_op == "or":
            //         cpu_op = "orr"
            //     if cpu_op == "xor":
            //         cpu_op = "eor"
            //     insns = insns.replace("OP", cpu_op)
            //     insns += fmt_insn("cmp %[scratch2], #1")
            //     insns += fmt_insn("beq 0b")
            //     insns += fmt_insn("dmb sy")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res;
            //             uintptr_t scratch1, scratch2;
            //             asm volatile (%(insns)s
            //                 : [res] "=&r" (res), [scratch1] "=&r" (scratch1), [scratch2] "=&r"(scratch2)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[expect(unreachable_code)]
        const { panic!("Unexpected arch") }
    };
    (u32, $op: tt, $ptr: ident, $val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // The `add` operation can be optimized with XADD.
            //     if op == "add":
            //         insns = ""
            //         if size == 8:
            //             insns += fmt_insn("lock; xaddb %[val], (%[addr])")
            //         elif size == 16:
            //             insns += fmt_insn("lock; xaddw %[val], (%[addr])")
            //         elif size == 32:
            //             insns += fmt_insn("lock; xaddl %[val], (%[addr])")
            //         else:
            //             assert size == 64
            //             insns += fmt_insn("lock; xaddq %[val], (%[addr])")
            //         return """
            //             INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //                 asm volatile (%(insns)s
            //                     : [val] "+&r" (val)
            //                     : [addr] "r" (addr)
            //                     : "memory", "cc");
            //                 return val;
            //             }""" % {
            //             "cpp_type": cpp_type,
            //             "fun_name": fun_name,
            //             "insns": insns,
            //         }
            //     // Use a +a constraint to ensure `res` is stored in RAX. This is required
            //     // for the CMPXCHG instruction.
            //     insns = ""
            //     if size == 8:
            //         insns += fmt_insn("movb (%[addr]), %[res]")
            //         insns += fmt_insn("0: movb %[res], %[scratch]")
            //         insns += fmt_insn("OPb %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgb %[scratch], (%[addr])")
            //     elif size == 16:
            //         insns += fmt_insn("movw (%[addr]), %[res]")
            //         insns += fmt_insn("0: movw %[res], %[scratch]")
            //         insns += fmt_insn("OPw %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgw %[scratch], (%[addr])")
            //     elif size == 32:
            //         insns += fmt_insn("movl (%[addr]), %[res]")
            //         insns += fmt_insn("0: movl %[res], %[scratch]")
            //         insns += fmt_insn("OPl %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgl %[scratch], (%[addr])")
            //     else:
            //         assert size == 64
            //         insns += fmt_insn("movq (%[addr]), %[res]")
            //         insns += fmt_insn("0: movq %[res], %[scratch]")
            //         insns += fmt_insn("OPq %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgq %[scratch], (%[addr])")
            //     insns = insns.replace("OP", op)
            //     insns += fmt_insn("jnz 0b")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res, scratch;
            //             asm volatile (%(insns)s
            //                 : [res] "=&a" (res), [scratch] "=&r" (scratch)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[cfg(target_arch = "aarch64")]
        {
            //     insns = ""
            //     insns += fmt_insn("dmb ish")
            //     insns += fmt_insn("0:")
            //     if size == 8:
            //         insns += fmt_insn("ldxrb %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxrb %w[scratch2], %w[scratch1], [%x[addr]]")
            //     elif size == 16:
            //         insns += fmt_insn("ldxrh %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxrh %w[scratch2], %w[scratch1], [%x[addr]]")
            //     elif size == 32:
            //         insns += fmt_insn("ldxr %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxr %w[scratch2], %w[scratch1], [%x[addr]]")
            //     else:
            //         assert size == 64
            //         insns += fmt_insn("ldxr %x[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxr %w[scratch2], %x[scratch1], [%x[addr]]")
            //     cpu_op = op
            //     if cpu_op == "or":
            //         cpu_op = "orr"
            //     if cpu_op == "xor":
            //         cpu_op = "eor"
            //     insns = insns.replace("OP", cpu_op)
            //     insns += fmt_insn("cbnz %w[scratch2], 0b")
            //     insns += fmt_insn("dmb ish")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res;
            //             uintptr_t scratch1, scratch2;
            //             asm volatile (%(insns)s
            //                 : [res] "=&r" (res), [scratch1] "=&r" (scratch1), [scratch2] "=&r"(scratch2)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            //     insns = ""
            //     insns += fmt_insn("dmb sy")
            //     insns += fmt_insn("0:")
            //     if size == 8:
            //         insns += fmt_insn("ldrexb %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strexb %[scratch2], %[scratch1], [%[addr]]")
            //     elif size == 16:
            //         insns += fmt_insn("ldrexh %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strexh %[scratch2], %[scratch1], [%[addr]]")
            //     else:
            //         assert size == 32
            //         insns += fmt_insn("ldrex %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strex %[scratch2], %[scratch1], [%[addr]]")
            //     cpu_op = op
            //     if cpu_op == "or":
            //         cpu_op = "orr"
            //     if cpu_op == "xor":
            //         cpu_op = "eor"
            //     insns = insns.replace("OP", cpu_op)
            //     insns += fmt_insn("cmp %[scratch2], #1")
            //     insns += fmt_insn("beq 0b")
            //     insns += fmt_insn("dmb sy")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res;
            //             uintptr_t scratch1, scratch2;
            //             asm volatile (%(insns)s
            //                 : [res] "=&r" (res), [scratch1] "=&r" (scratch1), [scratch2] "=&r"(scratch2)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[expect(unreachable_code)]
        const { panic!("Unexpected arch") }
    };
    (u64, $op: tt, $ptr: ident, $val: ident) => {
        // SAFETY: ptr is NonNull<()>; it is never null, dangling, or unaligned.
        let ptr = unsafe { &mut *$ptr.as_ptr() };

        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            // The `add` operation can be optimized with XADD.
            //     if op == "add":
            //         insns = ""
            //         if size == 8:
            //             insns += fmt_insn("lock; xaddb %[val], (%[addr])")
            //         elif size == 16:
            //             insns += fmt_insn("lock; xaddw %[val], (%[addr])")
            //         elif size == 32:
            //             insns += fmt_insn("lock; xaddl %[val], (%[addr])")
            //         else:
            //             assert size == 64
            //             insns += fmt_insn("lock; xaddq %[val], (%[addr])")
            //         return """
            //             INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //                 asm volatile (%(insns)s
            //                     : [val] "+&r" (val)
            //                     : [addr] "r" (addr)
            //                     : "memory", "cc");
            //                 return val;
            //             }""" % {
            //             "cpp_type": cpp_type,
            //             "fun_name": fun_name,
            //             "insns": insns,
            //         }
            //     // Use a +a constraint to ensure `res` is stored in RAX. This is required
            //     // for the CMPXCHG instruction.
            //     insns = ""
            //     if size == 8:
            //         insns += fmt_insn("movb (%[addr]), %[res]")
            //         insns += fmt_insn("0: movb %[res], %[scratch]")
            //         insns += fmt_insn("OPb %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgb %[scratch], (%[addr])")
            //     elif size == 16:
            //         insns += fmt_insn("movw (%[addr]), %[res]")
            //         insns += fmt_insn("0: movw %[res], %[scratch]")
            //         insns += fmt_insn("OPw %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgw %[scratch], (%[addr])")
            //     elif size == 32:
            //         insns += fmt_insn("movl (%[addr]), %[res]")
            //         insns += fmt_insn("0: movl %[res], %[scratch]")
            //         insns += fmt_insn("OPl %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgl %[scratch], (%[addr])")
            //     else:
            //         assert size == 64
            //         insns += fmt_insn("movq (%[addr]), %[res]")
            //         insns += fmt_insn("0: movq %[res], %[scratch]")
            //         insns += fmt_insn("OPq %[val], %[scratch]")
            //         insns += fmt_insn("lock; cmpxchgq %[scratch], (%[addr])")
            //     insns = insns.replace("OP", op)
            //     insns += fmt_insn("jnz 0b")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res, scratch;
            //             asm volatile (%(insns)s
            //                 : [res] "=&a" (res), [scratch] "=&r" (scratch)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[cfg(target_arch = "aarch64")]
        {
            //     insns = ""
            //     insns += fmt_insn("dmb ish")
            //     insns += fmt_insn("0:")
            //     if size == 8:
            //         insns += fmt_insn("ldxrb %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxrb %w[scratch2], %w[scratch1], [%x[addr]]")
            //     elif size == 16:
            //         insns += fmt_insn("ldxrh %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxrh %w[scratch2], %w[scratch1], [%x[addr]]")
            //     elif size == 32:
            //         insns += fmt_insn("ldxr %w[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxr %w[scratch2], %w[scratch1], [%x[addr]]")
            //     else:
            //         assert size == 64
            //         insns += fmt_insn("ldxr %x[res], [%x[addr]]")
            //         insns += fmt_insn("OP %x[scratch1], %x[res], %x[val]")
            //         insns += fmt_insn("stxr %w[scratch2], %x[scratch1], [%x[addr]]")
            //     cpu_op = op
            //     if cpu_op == "or":
            //         cpu_op = "orr"
            //     if cpu_op == "xor":
            //         cpu_op = "eor"
            //     insns = insns.replace("OP", cpu_op)
            //     insns += fmt_insn("cbnz %w[scratch2], 0b")
            //     insns += fmt_insn("dmb ish")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res;
            //             uintptr_t scratch1, scratch2;
            //             asm volatile (%(insns)s
            //                 : [res] "=&r" (res), [scratch1] "=&r" (scratch1), [scratch2] "=&r"(scratch2)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            //     insns = ""
            //     insns += fmt_insn("dmb sy")
            //     insns += fmt_insn("0:")
            //     if size == 8:
            //         insns += fmt_insn("ldrexb %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strexb %[scratch2], %[scratch1], [%[addr]]")
            //     elif size == 16:
            //         insns += fmt_insn("ldrexh %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strexh %[scratch2], %[scratch1], [%[addr]]")
            //     else:
            //         assert size == 32
            //         insns += fmt_insn("ldrex %[res], [%[addr]]")
            //         insns += fmt_insn("OP %[scratch1], %[res], %[val]")
            //         insns += fmt_insn("strex %[scratch2], %[scratch1], [%[addr]]")
            //     cpu_op = op
            //     if cpu_op == "or":
            //         cpu_op = "orr"
            //     if cpu_op == "xor":
            //         cpu_op = "eor"
            //     insns = insns.replace("OP", cpu_op)
            //     insns += fmt_insn("cmp %[scratch2], #1")
            //     insns += fmt_insn("beq 0b")
            //     insns += fmt_insn("dmb sy")
            //     return """
            //         INLINE_ATTR %(cpp_type)s %(fun_name)s(%(cpp_type)s* addr, %(cpp_type)s val) {
            //             %(cpp_type)s res;
            //             uintptr_t scratch1, scratch2;
            //             asm volatile (%(insns)s
            //                 : [res] "=&r" (res), [scratch1] "=&r" (scratch1), [scratch2] "=&r"(scratch2)
            //                 : [addr] "r" (addr), [val] "r"(val)
            //                 : "memory", "cc");
            //             return res;
            //         }""" % {
            //         "cpp_type": cpp_type,
            //         "fun_name": fun_name,
            //         "insns": insns,
            //     }
            todo!();
        }

        #[expect(unreachable_code)]
        const { panic!("Unexpected arch") }
    };
}

macro_rules! gen_copy {
    ($type: ty, $size: tt, $unroll: tt, $direction: tt) => {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            todo!();
        }

        #[cfg(target_arch = "aarch64")]
        {
            todo!();
        }

        #[cfg(target_arch = "arm")]
        unsafe {
            todo!();
        }

        #[expect(unreachable_code)]
        const {
            panic!("Unexpected arch")
        }
        // assert direction in ("down", "up")
        // offset = 0
        // if direction == "up":
        //     offset = unroll - 1
        // insns = ""
        // for i in range(unroll):
        //     if cpu_arch in ("x86", "x86_64"):
        //         if size == 1:
        //             insns += fmt_insn("movb OFFSET(%[src]), %[scratch]")
        //             insns += fmt_insn("movb %[scratch], OFFSET(%[dst])")
        //         elif size == 2:
        //             insns += fmt_insn("movw OFFSET(%[src]), %[scratch]")
        //             insns += fmt_insn("movw %[scratch], OFFSET(%[dst])")
        //         elif size == 4:
        //             insns += fmt_insn("movl OFFSET(%[src]), %[scratch]")
        //             insns += fmt_insn("movl %[scratch], OFFSET(%[dst])")
        //         else:
        //             assert size == 8
        //             insns += fmt_insn("movq OFFSET(%[src]), %[scratch]")
        //             insns += fmt_insn("movq %[scratch], OFFSET(%[dst])")
        //     elif cpu_arch == "aarch64":
        //         if size == 1:
        //             insns += fmt_insn("ldrb %w[scratch], [%x[src], OFFSET]")
        //             insns += fmt_insn("strb %w[scratch], [%x[dst], OFFSET]")
        //         elif size == 2:
        //             insns += fmt_insn("ldrh %w[scratch], [%x[src], OFFSET]")
        //             insns += fmt_insn("strh %w[scratch], [%x[dst], OFFSET]")
        //         elif size == 4:
        //             insns += fmt_insn("ldr %w[scratch], [%x[src], OFFSET]")
        //             insns += fmt_insn("str %w[scratch], [%x[dst], OFFSET]")
        //         else:
        //             assert size == 8
        //             insns += fmt_insn("ldr %x[scratch], [%x[src], OFFSET]")
        //             insns += fmt_insn("str %x[scratch], [%x[dst], OFFSET]")
        //     elif cpu_arch == "arm":
        //         if size == 1:
        //             insns += fmt_insn("ldrb %[scratch], [%[src], #OFFSET]")
        //             insns += fmt_insn("strb %[scratch], [%[dst], #OFFSET]")
        //         elif size == 2:
        //             insns += fmt_insn("ldrh %[scratch], [%[src], #OFFSET]")
        //             insns += fmt_insn("strh %[scratch], [%[dst], #OFFSET]")
        //         else:
        //             assert size == 4
        //             insns += fmt_insn("ldr %[scratch], [%[src], #OFFSET]")
        //             insns += fmt_insn("str %[scratch], [%[dst], #OFFSET]")
        //     else:
        //         raise Exception("Unexpected arch")
        //     insns = insns.replace("OFFSET", str(offset * size))

        //     if direction == "down":
        //         offset += 1
        //     else:
        //         offset -= 1

        // return """
        //     INLINE_ATTR void %(fun_name)s(uint8_t* dst, const uint8_t* src) {
        //         %(cpp_type)s* dst_ = reinterpret_cast<%(cpp_type)s*>(dst);
        //         const %(cpp_type)s* src_ = reinterpret_cast<const %(cpp_type)s*>(src);
        //         %(cpp_type)s scratch;
        //         asm volatile (%(insns)s
        //             : [scratch] "=&r" (scratch)
        //             : [dst] "r" (dst_), [src] "r"(src_)
        //             : "memory");
        //     }""" % {
        //     "cpp_type": cpp_type,
        //     "fun_name": fun_name,
        //     "insns": insns,
    };
}

/// ECMAScript atomic memory orderings
///
/// Memory orderings specify the way atomic operations synchronise memory.
/// With [`Ordering::Unordered`], no synchronisation is performed. With
/// [`Ordering::SeqCst`], a store-load pair of operations synchronize other
/// memory while additionally preserving a total order of such operations
/// across all threads.
///
/// The ECMAScript memory model is explained in the [ECMAScript Language
/// specification](https://tc39.es/ecma262/#sec-memory-model). Note that the
/// "INIT" ordering is not offered here as it is the purview of the memory
/// allocator.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum Ordering {
    Unordered,
    SeqCst,
}

/// A sequentially consistent atomic fence.
///
/// See [std::sync::atomic::fence] for details.
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn fence() {
    core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_load_8_seq_cst(ptr: NonNull<()>) -> u8 {
    gen_load!(u8, ptr, true);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_load_16_seq_cst(ptr: NonNull<()>) -> u16 {
    gen_load!(u16, ptr, true);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_load_32_seq_cst(ptr: NonNull<()>) -> u32 {
    gen_load!(u32, ptr, true);
}

// if is_64bit:
#[inline(always)]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64",))]
pub fn atomic_load_64_seq_cst(ptr: NonNull<()>) -> u64 {
    gen_load!(u64, ptr, true);
}

// These are access-atomic up to sizeof(uintptr_t).
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_load_8_unsynchronized(ptr: NonNull<()>) -> u8 {
    gen_load!(u8, ptr, false);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_load_16_unsynchronized(ptr: NonNull<()>) -> u16 {
    gen_load!(u16, ptr, false);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_load_32_unsynchronized(ptr: NonNull<()>) -> u32 {
    gen_load!(u32, ptr, false);
}

#[inline(always)]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64",))]
pub fn atomic_load_64_unsynchronized(ptr: NonNull<()>) -> u64 {
    gen_load!(u64, ptr, false);
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_store_8_seq_cst(ptr: NonNull<()>, val: u8) {
    gen_store!(u8, ptr, val, true);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_store_16_seq_cst(ptr: NonNull<()>, val: u16) {
    gen_store!(u16, ptr, val, true);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_store_32_seq_cst(ptr: NonNull<()>, val: u32) {
    gen_store!(u32, ptr, val, true);
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_store_64_seq_cst(ptr: NonNull<()>, val: u64) {
    gen_store!(u64, ptr, val, true);
}

// These are access-atomic up to sizeof(uintptr_t).
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_store_8_unsynchronized(ptr: NonNull<()>, val: u8) {
    gen_store!(u8, ptr, val, false);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_store_16_unsynchronized(ptr: NonNull<()>, val: u16) {
    gen_store!(u16, ptr, val, false);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_store_32_unsynchronized(ptr: NonNull<()>, val: u32) {
    gen_store!(u32, ptr, val, false);
}

#[inline(always)]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64",))]
pub fn atomic_store_64_unsynchronized(ptr: NonNull<()>, val: u64) {
    gen_store!(u64, ptr, val, false);
}

// `exchange` takes a cell address and a value.  It stores it in the cell and
// returns the value previously in the cell.
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_exchange_8_seq_cst(ptr: NonNull<()>, mut val: u8) -> u8 {
    gen_exchange!(u8, ptr, val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_exchange_16_seq_cst(ptr: NonNull<()>, mut val: u16) -> u16 {
    gen_exchange!(u16, ptr, val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_exchange_32_seq_cst(ptr: NonNull<()>, mut val: u32) -> u32 {
    gen_exchange!(u32, ptr, val);
}

#[inline(always)]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64",))]
pub fn atomic_exchange_64_seq_cst(ptr: NonNull<()>, mut val: u64) -> u64 {
    gen_exchange!(u64, ptr, val);
}

// `cmpxchg` takes a cell address, an expected value and a replacement value.
// If the value in the cell equals the expected value then the replacement value
// is stored in the cell.  It always returns the value previously in the cell.
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_cmp_xchg_8_seq_cst(ptr: NonNull<()>, mut old_val: u8, new_val: u8) -> u8 {
    gen_cmpxchg!(u8, ptr, old_val, new_val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_cmp_xchg_16_seq_cst(ptr: NonNull<()>, mut old_val: u16, new_val: u16) -> u16 {
    gen_cmpxchg!(u16, ptr, old_val, new_val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_cmp_xchg_32_seq_cst(ptr: NonNull<()>, mut old_val: u32, new_val: u32) -> u32 {
    gen_cmpxchg!(u32, ptr, old_val, new_val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_cmp_xchg_64_seq_cst(ptr: NonNull<()>, mut old_val: u64, new_val: u64) -> u64 {
    gen_cmpxchg!(u64, ptr, old_val, new_val);
}

// `add` adds a value atomically to the cell and returns the old value in the
// cell.  (There is no `sub`; just add the negated value.)
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_add_8_seq_cst(ptr: NonNull<()>, val: u8) -> u8 {
    gen_fetchop!(u8, "add", ptr, val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_add_16_seq_cst(ptr: NonNull<()>, val: u16) -> u16 {
    gen_fetchop!(u16, "add", ptr, val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_add_32_seq_cst(ptr: NonNull<()>, val: u32) -> u32 {
    gen_fetchop!(u32, "add", ptr, val);
}

#[inline(always)]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64",))]
pub fn atomic_add_64_seq_cst(ptr: NonNull<()>, val: u64) -> u64 {
    gen_fetchop!(u64, "add", ptr, val);
}

// `and` bitwise-and a value atomically into the cell and returns the old value
// in the cell.
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_and_8_seq_cst(ptr: NonNull<()>, val: u8) -> u8 {
    gen_fetchop!(u8, "and", ptr, val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_and_16_seq_cst(ptr: NonNull<()>, val: u16) -> u16 {
    gen_fetchop!(u16, "and", ptr, val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_and_32_seq_cst(ptr: NonNull<()>, val: u32) -> u32 {
    gen_fetchop!(u32, "and", ptr, val);
}

#[inline(always)]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64",))]
pub fn atomic_and_64_seq_cst(ptr: NonNull<()>, val: u64) -> u64 {
    gen_fetchop!(u64, "and", ptr, val);
}

// `or` bitwise-ors a value atomically into the cell and returns the old value
// in the cell.
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_or_8_seq_cst(ptr: NonNull<()>, val: u8) -> u8 {
    gen_fetchop!(u8, "or", ptr, val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_or_16_seq_cst(ptr: NonNull<()>, val: u16) -> u16 {
    gen_fetchop!(u16, "or", ptr, val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_or_32_seq_cst(ptr: NonNull<()>, val: u32) -> u32 {
    gen_fetchop!(u32, "or", ptr, val);
}

#[inline(always)]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64",))]
pub fn atomic_or_64_seq_cst(ptr: NonNull<()>, val: u64) -> u64 {
    gen_fetchop!(u64, "or", ptr, val);
}

// `xor` bitwise-xors a value atomically into the cell and returns the old value
// in the cell.
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_xor_8_seq_cst(ptr: NonNull<()>, val: u8) -> u8 {
    gen_fetchop!(u8, "xor", ptr, val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_xor_16_seq_cst(ptr: NonNull<()>, val: u16) -> u16 {
    gen_fetchop!(u16, "xor", ptr, val);
}
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_xor_32_seq_cst(ptr: NonNull<()>, val: u32) -> u32 {
    gen_fetchop!(u32, "xor", ptr, val);
}

// if is_64bit:
#[inline(always)]
#[cfg(any(target_arch = "x86_64", target_arch = "aarch64",))]
pub fn atomic_xor_64_seq_cst(ptr: NonNull<()>, val: u64) -> u64 {
    gen_fetchop!(u64, "xor", ptr, val);
}

/// Emits a machine instruction to signal the processor that it is running in a
/// busy-wait spin-loop (“spin lock”).
///
/// See [std::hint::spin_loop] for details.
#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_pause() {
    core::hint::spin_loop();
}

// See comment in jit/AtomicOperations-shared-jit.cpp for an explanation.
// wordsize = 8 if is_64bit else 4
// words_in_block = 8
// blocksize = words_in_block * wordsize

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_copy_unaligned_block_down_unsynchronized() {
    gen_copy!(u8, 1, blocksize, "down");
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_copy_unaligned_block_up_unsynchronized() {
    gen_copy!(u8, 1, blocksize, "up");
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_copy_unaligned_word_down_unsynchronized() {
    gen_copy!(u8, 1, wordsize, "down");
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_copy_unaligned_word_up_unsynchronized() {
    gen_copy!(u8, 1, wordsize, "up");
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_copy_block_down_unsynchronized() {
    gen_copy!(uptr, wordsize, words_in_block, "down");
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_copy_block_up_unsynchronized() {
    gen_copy!(uptr, wordsize, words_in_block, "up");
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_copy_word_unsynchronized() {
    gen_copy!(uptr, wordsize, 1, "down");
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_copy32_unsynchronized() {
    gen_copy!(u32, 4, 1, "down");
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_copy16_unsynchronized() {
    gen_copy!(u16, 2, 1, "down");
}

#[inline(always)]
#[cfg(any(
    target_arch = "x86",
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "arm"
))]
pub fn atomic_copy8_unsynchronized() {
    gen_copy!(u8, 1, 1, "down");
}

pub const JS_GENERATED_ATOMICS_BLOCKSIZE: usize = 0;
pub const JS_GENERATED_ATOMICS_WORSIZE: usize = 0;

#[test]
fn test_load() {
    let foo = NonNull::from(Box::leak(Box::new([0xFFFF_FFFF_FFFF_FFFFu64; 1]))).cast::<()>();

    assert_eq!(atomic_load_8_unsynchronized(foo), 0xFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);

    assert_eq!(atomic_load_16_unsynchronized(foo), 0xFFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);

    assert_eq!(atomic_load_32_unsynchronized(foo), 0xFFFF_FFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);

    assert_eq!(atomic_load_64_unsynchronized(foo), 0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);

    assert_eq!(atomic_load_8_seq_cst(foo), 0xFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);

    assert_eq!(atomic_load_16_seq_cst(foo), 0xFFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);

    assert_eq!(atomic_load_32_seq_cst(foo), 0xFFFF_FFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);

    assert_eq!(atomic_load_64_seq_cst(foo), 0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);

    let _ = unsafe { Box::from_raw(foo.cast::<u64>().as_ptr()) };
}

#[test]
fn test_store() {
    let foo = NonNull::from(Box::leak(Box::new([0u64; 1]))).cast::<()>();

    atomic_store_8_unsynchronized(foo, 0xFF);
    assert_eq!(atomic_load_8_unsynchronized(foo), 0xFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFF);

    atomic_store_16_unsynchronized(foo, 0xFFFF);
    assert_eq!(atomic_load_16_unsynchronized(foo), 0xFFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF);

    atomic_store_32_unsynchronized(foo, 0xFFFF_FFFF);
    assert_eq!(atomic_load_32_unsynchronized(foo), 0xFFFF_FFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF);

    atomic_store_64_unsynchronized(foo, 0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(atomic_load_64_unsynchronized(foo), 0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);

    atomic_store_64_unsynchronized(foo, 0x0);
    assert_eq!(atomic_load_64_unsynchronized(foo), 0x0);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0x0);

    atomic_store_8_seq_cst(foo, 0xFF);
    assert_eq!(atomic_load_8_seq_cst(foo), 0xFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFF);

    atomic_store_16_seq_cst(foo, 0xFFFF);
    assert_eq!(atomic_load_16_seq_cst(foo), 0xFFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF);

    atomic_store_32_seq_cst(foo, 0xFFFF_FFFF);
    assert_eq!(atomic_load_32_seq_cst(foo), 0xFFFF_FFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF);

    atomic_store_64_seq_cst(foo, 0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(atomic_load_64_seq_cst(foo), 0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);

    let _ = unsafe { Box::from_raw(foo.cast::<u64>().as_ptr()) };
}

#[test]
fn test_exchange() {
    let foo = NonNull::from(Box::leak(Box::new([0u64; 1]))).cast::<()>();

    assert_eq!(atomic_exchange_8_seq_cst(foo, 0xFF), 0, "u8 initial");
    assert_eq!(atomic_exchange_8_seq_cst(foo, 0), 0xFF, "u8 subsequent");
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);

    assert_eq!(atomic_exchange_16_seq_cst(foo, 0xFFFF), 0, "u16 initial");
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF);
    assert_eq!(atomic_exchange_16_seq_cst(foo, 0), 0xFFFF, "u16 subsequent");
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);

    assert_eq!(
        atomic_exchange_32_seq_cst(foo, 0xFFFF_FFFF),
        0,
        "u32 initial"
    );
    assert_eq!(
        atomic_exchange_32_seq_cst(foo, 0),
        0xFFFF_FFFF,
        "u32 subsequent"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);

    assert_eq!(
        atomic_exchange_64_seq_cst(foo, 0xFFFF_FFFF_FFFF_FFFF),
        0,
        "u64 initial"
    );
    assert_eq!(
        atomic_exchange_64_seq_cst(foo, 0),
        0xFFFF_FFFF_FFFF_FFFF,
        "u64 subsequent"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);

    let _ = unsafe { Box::from_raw(foo.cast::<u64>().as_ptr()) };
}

#[test]
fn test_compare_exchange() {
    let foo = NonNull::from(Box::leak(Box::new([0u64; 1]))).cast::<()>();

    assert_eq!(atomic_cmp_xchg_8_seq_cst(foo, 0xFF, 0xFF), 0, "u8 initial");
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);
    assert_eq!(atomic_cmp_xchg_8_seq_cst(foo, 0, 0xFF), 0, "u8 initial");
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFF);
    assert_eq!(atomic_cmp_xchg_8_seq_cst(foo, 0, 0), 0xFF, "u8 subsequent");
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFF);
    assert_eq!(
        atomic_cmp_xchg_8_seq_cst(foo, 0xFF, 0),
        0xFF,
        "u8 subsequent"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);

    assert_eq!(
        atomic_cmp_xchg_16_seq_cst(foo, 0xFFFF, 0xFFFF),
        0,
        "u16 initial"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);
    assert_eq!(atomic_cmp_xchg_16_seq_cst(foo, 0, 0xFFFF), 0, "u16 initial");
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF);
    assert_eq!(
        atomic_cmp_xchg_16_seq_cst(foo, 0, 0),
        0xFFFF,
        "u16 subsequent"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF);
    assert_eq!(
        atomic_cmp_xchg_16_seq_cst(foo, 0xFFFF, 0),
        0xFFFF,
        "u16 subsequent"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);

    assert_eq!(
        atomic_cmp_xchg_32_seq_cst(foo, 0xFFFF_FFFF, 0xFFFF_FFFF),
        0,
        "u32 initial"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);
    assert_eq!(
        atomic_cmp_xchg_32_seq_cst(foo, 0, 0xFFFF_FFFF),
        0,
        "u32 initial"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF);
    assert_eq!(
        atomic_cmp_xchg_32_seq_cst(foo, 0, 0),
        0xFFFF_FFFF,
        "u32 subsequent"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF);
    assert_eq!(
        atomic_cmp_xchg_32_seq_cst(foo, 0xFFFF_FFFF, 0),
        0xFFFF_FFFF,
        "u32 subsequent"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);

    assert_eq!(
        atomic_cmp_xchg_64_seq_cst(foo, 0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF),
        0,
        "u64 initial"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);
    assert_eq!(
        atomic_cmp_xchg_64_seq_cst(foo, 0, 0xFFFF_FFFF_FFFF_FFFF),
        0,
        "u64 initial"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(
        atomic_cmp_xchg_64_seq_cst(foo, 0, 0),
        0xFFFF_FFFF_FFFF_FFFF,
        "u64 subsequent"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0xFFFF_FFFF_FFFF_FFFF);
    assert_eq!(
        atomic_cmp_xchg_64_seq_cst(foo, 0xFFFF_FFFF_FFFF_FFFF, 0),
        0xFFFF_FFFF_FFFF_FFFF,
        "u64 subsequent"
    );
    assert_eq!(unsafe { foo.cast::<u64>().read() }, 0);

    let _ = unsafe { Box::from_raw(foo.cast::<u64>().as_ptr()) };
}
