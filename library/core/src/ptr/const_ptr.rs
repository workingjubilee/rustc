// FIXME(strict_provenance_magic): this module still uses lots of casts to polyfill things.
#![cfg_attr(not(bootstrap), allow(fuzzy_provenance_casts))]

use super::*;
use crate::cmp::Ordering::{self, Equal, Greater, Less};
use crate::intrinsics;
use crate::mem;
use crate::slice::{self, SliceIndex};

#[lang = "const_ptr"]
impl<T: ?Sized> *const T {
    /// Returns `true` if the pointer is null.
    ///
    /// Note that unsized types have many possible null pointers, as only the
    /// raw data pointer is considered, not their length, vtable, etc.
    /// Therefore, two pointers that are null may still not compare equal to
    /// each other.
    ///
    /// ## Behavior during const evaluation
    ///
    /// When this function is used during const evaluation, it may return `false` for pointers
    /// that turn out to be null at runtime. Specifically, when a pointer to some memory
    /// is offset beyond its bounds in such a way that the resulting pointer is null,
    /// the function will still return `false`. There is no way for CTFE to know
    /// the absolute position of that memory, so we cannot tell if the pointer is
    /// null or not.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let s: &str = "Follow the rabbit";
    /// let ptr: *const u8 = s.as_ptr();
    /// assert!(!ptr.is_null());
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    #[rustc_const_unstable(feature = "const_ptr_is_null", issue = "74939")]
    #[inline]
    pub const fn is_null(self) -> bool {
        // Compare via a cast to a thin pointer, so fat pointers are only
        // considering their "data" part for null-ness.
        (self as *const u8).guaranteed_eq(null())
    }

    /// Casts to a pointer of another type.
    #[stable(feature = "ptr_cast", since = "1.38.0")]
    #[rustc_const_stable(feature = "const_ptr_cast", since = "1.38.0")]
    #[inline]
    pub const fn cast<U>(self) -> *const U {
        self as _
    }

    /// Changes constness without changing the type.
    ///
    /// This is a bit safer than `as` because it wouldn't silently change the type if the code is
    /// refactored.
    #[unstable(feature = "ptr_const_cast", issue = "92675")]
    #[rustc_const_unstable(feature = "ptr_const_cast", issue = "92675")]
    pub const fn as_mut(self) -> *mut T {
        self as _
    }

    /// Casts a pointer to its raw bits.
    ///
    /// In general, pointers cannot be understood as "just an integer"
    /// and cannot be created from one without additional context.
    ///
    /// If you would like to treat a pointer like an integer anyway,
    /// see [`addr`][#method.addr-1] and [`with_addr`][#method.with_addr-1] for the responsible
    /// way to do that.
    #[unstable(feature = "ptr_to_from_bits", issue = "91126")]
    pub fn to_bits(self) -> [u8; core::mem::size_of::<*const ()>()]
    where
        T: Sized,
    {
        // SAFETY: I AM THE MAGIC
        unsafe { core::mem::transmute(self) }
    }

    /// Creates a pointer from its raw bits.
    ///
    /// This is equivalent to `as *const T`, but is more specific to enhance readability.
    /// The inverse method is [`to_bits`](#method.to_bits-1).
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(ptr_to_from_bits)]
    /// use std::ptr::NonNull;
    /// let dangling: *mut u8 = NonNull::dangling().as_ptr();
    /// assert_eq!(<*mut u8>::from_bits(1), dangling);
    /// ```
    #[rustc_deprecated(
        since = "1.61.0",
        reason = "This design is incompatible with Pointer Provenance",
        suggestion = "from_addr"
    )]
    #[unstable(feature = "ptr_to_from_bits", issue = "91126")]
    pub fn from_bits(bits: usize) -> Self
    where
        T: Sized,
    {
        bits as Self
    }

    /// Gets the "address" portion of the pointer.
    ///
    /// On most platforms this is a no-op, as the pointer is just an address,
    /// and is equivalent to the deprecated `ptr as usize` cast.
    ///
    /// On more complicated platforms like CHERI and segmented architectures,
    /// this may remove some important metadata. See [`with_addr`][#method.with_addr-1] for
    /// details on this distinction and why it's important.
    #[unstable(feature = "strict_provenance", issue = "99999999")]
    pub fn addr(self) -> usize
    where
        T: Sized,
    {
        // FIXME(strict_provenance_magic): I am magic and should be a compiler intrinsic.
        self as usize
    }

    /// Creates a new pointer with the given address.
    ///
    /// This replaces the deprecated `usize as ptr` cast, which had
    /// fundamentally broken semantics because it couldn't restore
    /// *segment* and *provenance*.
    ///
    /// A pointer semantically has 3 pieces of information associated with it:
    ///
    /// * Segment: The address-space it is part of.
    /// * Provenance: An allocation (slice) that it is allowed to access.
    /// * Address: The actual address it points at.
    ///
    /// The compiler and hardware need to properly understand all 3 of these
    /// values at all times to properly execute your code.
    ///
    /// Segment and Provenance are implicitly defined by *how* a pointer is
    /// constructed and generally propagates verbatim to all derived pointers.
    /// It is therefore *impossible* to convert an address into a pointer
    /// on its own, because there is no way to know what its segment and
    /// provenance should be.
    ///
    /// By introducing a "representative" pointer into the process we can
    /// properly construct a new pointer with *its* segment and provenance,
    /// just as any other derived pointer would. This *should* be equivalent
    /// to `wrapping_offset`ting the given pointer to the new address. See the
    /// docs for `wrapping_offset` for the restrictions this applies.
    ///
    /// # Example
    ///
    /// Here is an example of how to properly use this API to mess around
    /// with tagged pointers. Here we have a tag in the lowest bit:
    ///
    /// ```text
    /// let my_tagged_ptr: *const T = ...;
    ///
    /// // Get the address and do whatever bit tricks we like
    /// let addr = my_tagged_ptr.addr();
    /// let has_tag = (addr & 0x1) != 0;
    /// let real_addr = addr & !0x1;
    ///
    /// // Reconstitute a pointer with the new address and use it
    /// let my_untagged_ptr = my_tagged_ptr.with_addr(real_addr);
    /// let val = *my_untagged_ptr;
    /// ```
    #[unstable(feature = "strict_provenance", issue = "99999999")]
    pub fn with_addr(self, addr: usize) -> Self
    where
        T: Sized,
    {
        // FIXME(strict_provenance_magic): I am magic and should be a compiler intrinsic.
        //
        // In the mean-time, this operation is defined to be "as if" it was
        // a wrapping_offset, so we can emulate it as such. This should properly
        // restore pointer provenance even under today's compiler.
        let self_addr = self.addr() as isize;
        let dest_addr = addr as isize;
        let offset = dest_addr.wrapping_sub(self_addr);

        // This is the canonical desugarring of this operation
        self.cast::<u8>().wrapping_offset(offset).cast::<T>()
    }

    /// Decompose a (possibly wide) pointer into its address and metadata components.
    ///
    /// The pointer can be later reconstructed with [`from_raw_parts`].
    #[unstable(feature = "ptr_metadata", issue = "81513")]
    #[rustc_const_unstable(feature = "ptr_metadata", issue = "81513")]
    #[inline]
    pub const fn to_raw_parts(self) -> (*const (), <T as super::Pointee>::Metadata) {
        (self.cast(), metadata(self))
    }

    /// Returns `None` if the pointer is null, or else returns a shared reference to
    /// the value wrapped in `Some`. If the value may be uninitialized, [`as_uninit_ref`]
    /// must be used instead.
    ///
    /// [`as_uninit_ref`]: #method.as_uninit_ref
    ///
    /// # Safety
    ///
    /// When calling this method, you have to ensure that *either* the pointer is null *or*
    /// all of the following is true:
    ///
    /// * The pointer must be properly aligned.
    ///
    /// * It must be "dereferenceable" in the sense defined in [the module documentation].
    ///
    /// * The pointer must point to an initialized instance of `T`.
    ///
    /// * You must enforce Rust's aliasing rules, since the returned lifetime `'a` is
    ///   arbitrarily chosen and does not necessarily reflect the actual lifetime of the data.
    ///   In particular, for the duration of this lifetime, the memory the pointer points to must
    ///   not get mutated (except inside `UnsafeCell`).
    ///
    /// This applies even if the result of this method is unused!
    /// (The part about being initialized is not yet fully decided, but until
    /// it is, the only safe approach is to ensure that they are indeed initialized.)
    ///
    /// [the module documentation]: crate::ptr#safety
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let ptr: *const u8 = &10u8 as *const u8;
    ///
    /// unsafe {
    ///     if let Some(val_back) = ptr.as_ref() {
    ///         println!("We got back the value: {val_back}!");
    ///     }
    /// }
    /// ```
    ///
    /// # Null-unchecked version
    ///
    /// If you are sure the pointer can never be null and are looking for some kind of
    /// `as_ref_unchecked` that returns the `&T` instead of `Option<&T>`, know that you can
    /// dereference the pointer directly.
    ///
    /// ```
    /// let ptr: *const u8 = &10u8 as *const u8;
    ///
    /// unsafe {
    ///     let val_back = &*ptr;
    ///     println!("We got back the value: {val_back}!");
    /// }
    /// ```
    #[stable(feature = "ptr_as_ref", since = "1.9.0")]
    #[rustc_const_unstable(feature = "const_ptr_as_ref", issue = "91822")]
    #[inline]
    pub const unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        // SAFETY: the caller must guarantee that `self` is valid
        // for a reference if it isn't null.
        if self.is_null() { None } else { unsafe { Some(&*self) } }
    }

    /// Returns `None` if the pointer is null, or else returns a shared reference to
    /// the value wrapped in `Some`. In contrast to [`as_ref`], this does not require
    /// that the value has to be initialized.
    ///
    /// [`as_ref`]: #method.as_ref
    ///
    /// # Safety
    ///
    /// When calling this method, you have to ensure that *either* the pointer is null *or*
    /// all of the following is true:
    ///
    /// * The pointer must be properly aligned.
    ///
    /// * It must be "dereferenceable" in the sense defined in [the module documentation].
    ///
    /// * You must enforce Rust's aliasing rules, since the returned lifetime `'a` is
    ///   arbitrarily chosen and does not necessarily reflect the actual lifetime of the data.
    ///   In particular, for the duration of this lifetime, the memory the pointer points to must
    ///   not get mutated (except inside `UnsafeCell`).
    ///
    /// This applies even if the result of this method is unused!
    ///
    /// [the module documentation]: crate::ptr#safety
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// #![feature(ptr_as_uninit)]
    ///
    /// let ptr: *const u8 = &10u8 as *const u8;
    ///
    /// unsafe {
    ///     if let Some(val_back) = ptr.as_uninit_ref() {
    ///         println!("We got back the value: {}!", val_back.assume_init());
    ///     }
    /// }
    /// ```
    #[inline]
    #[unstable(feature = "ptr_as_uninit", issue = "75402")]
    #[rustc_const_unstable(feature = "const_ptr_as_ref", issue = "91822")]
    pub const unsafe fn as_uninit_ref<'a>(self) -> Option<&'a MaybeUninit<T>>
    where
        T: Sized,
    {
        // SAFETY: the caller must guarantee that `self` meets all the
        // requirements for a reference.
        if self.is_null() { None } else { Some(unsafe { &*(self as *const MaybeUninit<T>) }) }
    }

    /// Calculates the offset from a pointer.
    ///
    /// `count` is in units of T; e.g., a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and resulting pointer must be either in bounds or one
    ///   byte past the end of the same [allocated object].
    ///
    /// * The computed offset, **in bytes**, cannot overflow an `isize`.
    ///
    /// * The offset being in bounds cannot rely on "wrapping around" the address
    ///   space. That is, the infinite-precision sum, **in bytes** must fit in a usize.
    ///
    /// The compiler and standard library generally tries to ensure allocations
    /// never reach a size where an offset is a concern. For instance, `Vec`
    /// and `Box` ensure they never allocate more than `isize::MAX` bytes, so
    /// `vec.as_ptr().add(vec.len())` is always safe.
    ///
    /// Most platforms fundamentally can't even construct such an allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    ///
    /// Consider using [`wrapping_offset`] instead if these constraints are
    /// difficult to satisfy. The only advantage of this method is that it
    /// enables more aggressive compiler optimizations.
    ///
    /// [`wrapping_offset`]: #method.wrapping_offset
    /// [allocated object]: crate::ptr#allocated-object
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let s: &str = "123";
    /// let ptr: *const u8 = s.as_ptr();
    ///
    /// unsafe {
    ///     println!("{}", *ptr.offset(1) as char);
    ///     println!("{}", *ptr.offset(2) as char);
    /// }
    /// ```
    #[stable(feature = "rust1", since = "1.0.0")]
    #[must_use = "returns a new pointer rather than modifying its argument"]
    #[rustc_const_unstable(feature = "const_ptr_offset", issue = "71499")]
    #[inline(always)]
    pub const unsafe fn offset(self, count: isize) -> *const T
    where
        T: Sized,
    {
        // SAFETY: the caller must uphold the safety contract for `offset`.
        unsafe { intrinsics::offset(self, count) }
    }

    /// Calculates the offset from a pointer using wrapping arithmetic.
    ///
    /// `count` is in units of T; e.g., a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// This operation itself is always safe, but using the resulting pointer is not.
    ///
    /// The resulting pointer "remembers" the [allocated object] that `self` points to; it must not
    /// be used to read or write other allocated objects. This is tracked by provenance.
    ///
    /// In other words, `let z = x.wrapping_offset((y.addr() as isize) - (x.addr() as isize))`
    /// does *not* make `z` the same as `y` even if we assume `T` has size `1` and there is no overflow: `z` is still
    /// attached to the object `x` is attached to, and dereferencing it is Undefined Behavior unless
    /// `x` and `y` point into the same allocated object.
    ///
    /// Compared to [`offset`], this method basically delays the requirement of staying within the
    /// same allocated object: [`offset`] is immediate Undefined Behavior when crossing object
    /// boundaries; `wrapping_offset` produces a pointer but still leads to Undefined Behavior if a
    /// pointer is dereferenced when it is out-of-bounds of the object it is attached to. [`offset`]
    /// can be optimized better and is thus preferable in performance-sensitive code.
    ///
    /// The delayed check only considers the value of the pointer that was dereferenced, not the
    /// intermediate values used during the computation of the final result. For example,
    /// `x.wrapping_offset(o).wrapping_offset(o.wrapping_neg())` is always the same as `x`...
    ///
    /// Usually.
    ///
    /// More work needs to be done to define the rules here, but on CHERI it is not *actually*
    /// a no-op to wrapping_offset a pointer to some random address and back again. For practical
    /// applications that actually need this, it *will* generally work, but if your offset is
    /// "too out of bounds" the system will mark your pointer as invalid, and subsequent reads
    /// will fault *as if* the pointer had been corrupted by a non-pointer instruction.
    ///
    /// CHERI has a roughly 64-bit address space but its 128-bit pointers contain
    /// 3 ostensibly-address-space-sized values:
    ///
    /// * 2 values for the "slice" that the pointer can access.
    /// * 1 value for the actuall address it points to.
    ///
    /// To accomplish this, CHERI compresses the values and even requires large allocations
    /// to have higher alignment to free up extra bits. This compression scheme can support
    /// the pointer being offset outside of the slice, but only to an extent. A *generous*
    /// extent, but a limited one nonetheless. To quote CHERI's documenation:
    ///
    /// > With 27 bits of the capability used for bounds, CHERI-MIPS and 64-bit
    /// > CHERI-RISC-V provide the following guarantees:
    /// >
    /// > * A pointer is able to travel at least 1⁄4 the size of the object, or 2 KiB,
    /// > whichever is greater, above its upper bound.
    /// > * It is able to travel at least 1⁄8 the size of the object, or 1 KiB,
    /// > whichever is greater, below its lower bound.
    ///
    /// Needless to say, any scheme that relies on reusing the least significant bits
    /// of a pointer based on alignment is going to be fine. Any scheme which tries
    /// to set *high* bits isn't going to work, but that was *already* extremely
    /// platform-specific and not at all portable.
    ///
    /// [`offset`]: #method.offset
    /// [allocated object]: crate::ptr#allocated-object
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// // Iterate using a raw pointer in increments of two elements
    /// let data = [1u8, 2, 3, 4, 5];
    /// let mut ptr: *const u8 = data.as_ptr();
    /// let step = 2;
    /// let end_rounded_up = ptr.wrapping_offset(6);
    ///
    /// // This loop prints "1, 3, 5, "
    /// while ptr != end_rounded_up {
    ///     unsafe {
    ///         print!("{}, ", *ptr);
    ///     }
    ///     ptr = ptr.wrapping_offset(step);
    /// }
    /// ```
    #[stable(feature = "ptr_wrapping_offset", since = "1.16.0")]
    #[must_use = "returns a new pointer rather than modifying its argument"]
    #[rustc_const_unstable(feature = "const_ptr_offset", issue = "71499")]
    #[inline(always)]
    pub const fn wrapping_offset(self, count: isize) -> *const T
    where
        T: Sized,
    {
        // SAFETY: the `arith_offset` intrinsic has no prerequisites to be called.
        unsafe { intrinsics::arith_offset(self, count) }
    }

    /// Calculates the distance between two pointers. The returned value is in
    /// units of T: the distance in bytes is divided by `mem::size_of::<T>()`.
    ///
    /// This function is the inverse of [`offset`].
    ///
    /// [`offset`]: #method.offset
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and other pointer must be either in bounds or one
    ///   byte past the end of the same [allocated object].
    ///
    /// * Both pointers must be *derived from* a pointer to the same object.
    ///   (See below for an example.)
    ///
    /// * The distance between the pointers, in bytes, must be an exact multiple
    ///   of the size of `T`.
    ///
    /// * The distance between the pointers, **in bytes**, cannot overflow an `isize`.
    ///
    /// * The distance being in bounds cannot rely on "wrapping around" the address space.
    ///
    /// Rust types are never larger than `isize::MAX` and Rust allocations never wrap around the
    /// address space, so two pointers within some value of any Rust type `T` will always satisfy
    /// the last two conditions. The standard library also generally ensures that allocations
    /// never reach a size where an offset is a concern. For instance, `Vec` and `Box` ensure they
    /// never allocate more than `isize::MAX` bytes, so `ptr_into_vec.offset_from(vec.as_ptr())`
    /// always satisfies the last two conditions.
    ///
    /// Most platforms fundamentally can't even construct such a large allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    /// (Note that [`offset`] and [`add`] also have a similar limitation and hence cannot be used on
    /// such large allocations either.)
    ///
    /// [`add`]: #method.add
    /// [allocated object]: crate::ptr#allocated-object
    ///
    /// # Panics
    ///
    /// This function panics if `T` is a Zero-Sized Type ("ZST").
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let a = [0; 5];
    /// let ptr1: *const i32 = &a[1];
    /// let ptr2: *const i32 = &a[3];
    /// unsafe {
    ///     assert_eq!(ptr2.offset_from(ptr1), 2);
    ///     assert_eq!(ptr1.offset_from(ptr2), -2);
    ///     assert_eq!(ptr1.offset(2), ptr2);
    ///     assert_eq!(ptr2.offset(-2), ptr1);
    /// }
    /// ```
    ///
    /// *Incorrect* usage:
    ///
    /// ```rust,no_run
    /// let ptr1 = Box::into_raw(Box::new(0u8)) as *const u8;
    /// let ptr2 = Box::into_raw(Box::new(1u8)) as *const u8;
    /// let diff = (ptr2.addr() as isize).wrapping_sub(ptr1.addr() as isize);
    /// // Make ptr2_other an "alias" of ptr2, but derived from ptr1.
    /// let ptr2_other = (ptr1 as *const u8).wrapping_offset(diff);
    /// assert_eq!(ptr2.addr(), ptr2_other.addr());
    /// // Since ptr2_other and ptr2 are derived from pointers to different objects,
    /// // computing their offset is undefined behavior, even though
    /// // they point to the same address!
    /// unsafe {
    ///     let zero = ptr2_other.offset_from(ptr2); // Undefined Behavior
    /// }
    /// ```
    #[stable(feature = "ptr_offset_from", since = "1.47.0")]
    #[rustc_const_unstable(feature = "const_ptr_offset_from", issue = "92980")]
    #[inline]
    pub const unsafe fn offset_from(self, origin: *const T) -> isize
    where
        T: Sized,
    {
        let pointee_size = mem::size_of::<T>();
        assert!(0 < pointee_size && pointee_size <= isize::MAX as usize);
        // SAFETY: the caller must uphold the safety contract for `ptr_offset_from`.
        unsafe { intrinsics::ptr_offset_from(self, origin) }
    }

    /// Returns whether two pointers are guaranteed to be equal.
    ///
    /// At runtime this function behaves like `self == other`.
    /// However, in some contexts (e.g., compile-time evaluation),
    /// it is not always possible to determine equality of two pointers, so this function may
    /// spuriously return `false` for pointers that later actually turn out to be equal.
    /// But when it returns `true`, the pointers are guaranteed to be equal.
    ///
    /// This function is the mirror of [`guaranteed_ne`], but not its inverse. There are pointer
    /// comparisons for which both functions return `false`.
    ///
    /// [`guaranteed_ne`]: #method.guaranteed_ne
    ///
    /// The return value may change depending on the compiler version and unsafe code might not
    /// rely on the result of this function for soundness. It is suggested to only use this function
    /// for performance optimizations where spurious `false` return values by this function do not
    /// affect the outcome, but just the performance.
    /// The consequences of using this method to make runtime and compile-time code behave
    /// differently have not been explored. This method should not be used to introduce such
    /// differences, and it should also not be stabilized before we have a better understanding
    /// of this issue.
    #[unstable(feature = "const_raw_ptr_comparison", issue = "53020")]
    #[rustc_const_unstable(feature = "const_raw_ptr_comparison", issue = "53020")]
    #[inline]
    pub const fn guaranteed_eq(self, other: *const T) -> bool
    where
        T: Sized,
    {
        intrinsics::ptr_guaranteed_eq(self, other)
    }

    /// Returns whether two pointers are guaranteed to be unequal.
    ///
    /// At runtime this function behaves like `self != other`.
    /// However, in some contexts (e.g., compile-time evaluation),
    /// it is not always possible to determine the inequality of two pointers, so this function may
    /// spuriously return `false` for pointers that later actually turn out to be unequal.
    /// But when it returns `true`, the pointers are guaranteed to be unequal.
    ///
    /// This function is the mirror of [`guaranteed_eq`], but not its inverse. There are pointer
    /// comparisons for which both functions return `false`.
    ///
    /// [`guaranteed_eq`]: #method.guaranteed_eq
    ///
    /// The return value may change depending on the compiler version and unsafe code might not
    /// rely on the result of this function for soundness. It is suggested to only use this function
    /// for performance optimizations where spurious `false` return values by this function do not
    /// affect the outcome, but just the performance.
    /// The consequences of using this method to make runtime and compile-time code behave
    /// differently have not been explored. This method should not be used to introduce such
    /// differences, and it should also not be stabilized before we have a better understanding
    /// of this issue.
    #[unstable(feature = "const_raw_ptr_comparison", issue = "53020")]
    #[rustc_const_unstable(feature = "const_raw_ptr_comparison", issue = "53020")]
    #[inline]
    pub const fn guaranteed_ne(self, other: *const T) -> bool
    where
        T: Sized,
    {
        intrinsics::ptr_guaranteed_ne(self, other)
    }

    /// Calculates the offset from a pointer (convenience for `.offset(count as isize)`).
    ///
    /// `count` is in units of T; e.g., a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and resulting pointer must be either in bounds or one
    ///   byte past the end of the same [allocated object].
    ///
    /// * The computed offset, **in bytes**, cannot overflow an `isize`.
    ///
    /// * The offset being in bounds cannot rely on "wrapping around" the address
    ///   space. That is, the infinite-precision sum must fit in a `usize`.
    ///
    /// The compiler and standard library generally tries to ensure allocations
    /// never reach a size where an offset is a concern. For instance, `Vec`
    /// and `Box` ensure they never allocate more than `isize::MAX` bytes, so
    /// `vec.as_ptr().add(vec.len())` is always safe.
    ///
    /// Most platforms fundamentally can't even construct such an allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    ///
    /// Consider using [`wrapping_add`] instead if these constraints are
    /// difficult to satisfy. The only advantage of this method is that it
    /// enables more aggressive compiler optimizations.
    ///
    /// [`wrapping_add`]: #method.wrapping_add
    /// [allocated object]: crate::ptr#allocated-object
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let s: &str = "123";
    /// let ptr: *const u8 = s.as_ptr();
    ///
    /// unsafe {
    ///     println!("{}", *ptr.add(1) as char);
    ///     println!("{}", *ptr.add(2) as char);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[must_use = "returns a new pointer rather than modifying its argument"]
    #[rustc_const_unstable(feature = "const_ptr_offset", issue = "71499")]
    #[inline(always)]
    pub const unsafe fn add(self, count: usize) -> Self
    where
        T: Sized,
    {
        // SAFETY: the caller must uphold the safety contract for `offset`.
        unsafe { self.offset(count as isize) }
    }

    /// Calculates the offset from a pointer (convenience for
    /// `.offset((count as isize).wrapping_neg())`).
    ///
    /// `count` is in units of T; e.g., a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// If any of the following conditions are violated, the result is Undefined
    /// Behavior:
    ///
    /// * Both the starting and resulting pointer must be either in bounds or one
    ///   byte past the end of the same [allocated object].
    ///
    /// * The computed offset cannot exceed `isize::MAX` **bytes**.
    ///
    /// * The offset being in bounds cannot rely on "wrapping around" the address
    ///   space. That is, the infinite-precision sum must fit in a usize.
    ///
    /// The compiler and standard library generally tries to ensure allocations
    /// never reach a size where an offset is a concern. For instance, `Vec`
    /// and `Box` ensure they never allocate more than `isize::MAX` bytes, so
    /// `vec.as_ptr().add(vec.len()).sub(vec.len())` is always safe.
    ///
    /// Most platforms fundamentally can't even construct such an allocation.
    /// For instance, no known 64-bit platform can ever serve a request
    /// for 2<sup>63</sup> bytes due to page-table limitations or splitting the address space.
    /// However, some 32-bit and 16-bit platforms may successfully serve a request for
    /// more than `isize::MAX` bytes with things like Physical Address
    /// Extension. As such, memory acquired directly from allocators or memory
    /// mapped files *may* be too large to handle with this function.
    ///
    /// Consider using [`wrapping_sub`] instead if these constraints are
    /// difficult to satisfy. The only advantage of this method is that it
    /// enables more aggressive compiler optimizations.
    ///
    /// [`wrapping_sub`]: #method.wrapping_sub
    /// [allocated object]: crate::ptr#allocated-object
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// let s: &str = "123";
    ///
    /// unsafe {
    ///     let end: *const u8 = s.as_ptr().add(3);
    ///     println!("{}", *end.sub(1) as char);
    ///     println!("{}", *end.sub(2) as char);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[must_use = "returns a new pointer rather than modifying its argument"]
    #[rustc_const_unstable(feature = "const_ptr_offset", issue = "71499")]
    #[inline]
    pub const unsafe fn sub(self, count: usize) -> Self
    where
        T: Sized,
    {
        // SAFETY: the caller must uphold the safety contract for `offset`.
        unsafe { self.offset((count as isize).wrapping_neg()) }
    }

    /// Calculates the offset from a pointer using wrapping arithmetic.
    /// (convenience for `.wrapping_offset(count as isize)`)
    ///
    /// `count` is in units of T; e.g., a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// This operation itself is always safe, but using the resulting pointer is not.
    ///
    /// The resulting pointer "remembers" the [allocated object] that `self` points to; it must not
    /// be used to read or write other allocated objects.
    ///
    /// In other words, `let z = x.wrapping_add((y.addr()) - (x.addr()))` does *not* make `z`
    /// the same as `y` even if we assume `T` has size `1` and there is no overflow: `z` is still
    /// attached to the object `x` is attached to, and dereferencing it is Undefined Behavior unless
    /// `x` and `y` point into the same allocated object.
    ///
    /// Compared to [`add`], this method basically delays the requirement of staying within the
    /// same allocated object: [`add`] is immediate Undefined Behavior when crossing object
    /// boundaries; `wrapping_add` produces a pointer but still leads to Undefined Behavior if a
    /// pointer is dereferenced when it is out-of-bounds of the object it is attached to. [`add`]
    /// can be optimized better and is thus preferable in performance-sensitive code.
    ///
    /// The delayed check only considers the value of the pointer that was dereferenced, not the
    /// intermediate values used during the computation of the final result. For example,
    /// `x.wrapping_add(o).wrapping_sub(o)` is always the same as `x`. In other words, leaving the
    /// allocated object and then re-entering it later is permitted.
    ///
    /// [`add`]: #method.add
    /// [allocated object]: crate::ptr#allocated-object
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// // Iterate using a raw pointer in increments of two elements
    /// let data = [1u8, 2, 3, 4, 5];
    /// let mut ptr: *const u8 = data.as_ptr();
    /// let step = 2;
    /// let end_rounded_up = ptr.wrapping_add(6);
    ///
    /// // This loop prints "1, 3, 5, "
    /// while ptr != end_rounded_up {
    ///     unsafe {
    ///         print!("{}, ", *ptr);
    ///     }
    ///     ptr = ptr.wrapping_add(step);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[must_use = "returns a new pointer rather than modifying its argument"]
    #[rustc_const_unstable(feature = "const_ptr_offset", issue = "71499")]
    #[inline(always)]
    pub const fn wrapping_add(self, count: usize) -> Self
    where
        T: Sized,
    {
        self.wrapping_offset(count as isize)
    }

    /// Calculates the offset from a pointer using wrapping arithmetic.
    /// (convenience for `.wrapping_offset((count as isize).wrapping_neg())`)
    ///
    /// `count` is in units of T; e.g., a `count` of 3 represents a pointer
    /// offset of `3 * size_of::<T>()` bytes.
    ///
    /// # Safety
    ///
    /// This operation itself is always safe, but using the resulting pointer is not.
    ///
    /// The resulting pointer "remembers" the [allocated object] that `self` points to; it must not
    /// be used to read or write other allocated objects.
    ///
    /// In other words, `let z = x.wrapping_sub((x.addr()) - (y.addr()))` does *not* make `z`
    /// the same as `y` even if we assume `T` has size `1` and there is no overflow: `z` is still
    /// attached to the object `x` is attached to, and dereferencing it is Undefined Behavior unless
    /// `x` and `y` point into the same allocated object.
    ///
    /// Compared to [`sub`], this method basically delays the requirement of staying within the
    /// same allocated object: [`sub`] is immediate Undefined Behavior when crossing object
    /// boundaries; `wrapping_sub` produces a pointer but still leads to Undefined Behavior if a
    /// pointer is dereferenced when it is out-of-bounds of the object it is attached to. [`sub`]
    /// can be optimized better and is thus preferable in performance-sensitive code.
    ///
    /// The delayed check only considers the value of the pointer that was dereferenced, not the
    /// intermediate values used during the computation of the final result. For example,
    /// `x.wrapping_add(o).wrapping_sub(o)` is always the same as `x`. In other words, leaving the
    /// allocated object and then re-entering it later is permitted.
    ///
    /// [`sub`]: #method.sub
    /// [allocated object]: crate::ptr#allocated-object
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// // Iterate using a raw pointer in increments of two elements (backwards)
    /// let data = [1u8, 2, 3, 4, 5];
    /// let mut ptr: *const u8 = data.as_ptr();
    /// let start_rounded_down = ptr.wrapping_sub(2);
    /// ptr = ptr.wrapping_add(4);
    /// let step = 2;
    /// // This loop prints "5, 3, 1, "
    /// while ptr != start_rounded_down {
    ///     unsafe {
    ///         print!("{}, ", *ptr);
    ///     }
    ///     ptr = ptr.wrapping_sub(step);
    /// }
    /// ```
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[must_use = "returns a new pointer rather than modifying its argument"]
    #[rustc_const_unstable(feature = "const_ptr_offset", issue = "71499")]
    #[inline]
    pub const fn wrapping_sub(self, count: usize) -> Self
    where
        T: Sized,
    {
        self.wrapping_offset((count as isize).wrapping_neg())
    }

    /// Sets the pointer value to `ptr`.
    ///
    /// In case `self` is a (fat) pointer to an unsized type, this operation
    /// will only affect the pointer part, whereas for (thin) pointers to
    /// sized types, this has the same effect as a simple assignment.
    ///
    /// The resulting pointer will have provenance of `val`, i.e., for a fat
    /// pointer, this operation is semantically the same as creating a new
    /// fat pointer with the data pointer value of `val` but the metadata of
    /// `self`.
    ///
    /// # Examples
    ///
    /// This function is primarily useful for allowing byte-wise pointer
    /// arithmetic on potentially fat pointers:
    ///
    /// ```
    /// #![feature(set_ptr_value)]
    /// # use core::fmt::Debug;
    /// let arr: [i32; 3] = [1, 2, 3];
    /// let mut ptr = arr.as_ptr() as *const dyn Debug;
    /// let thin = ptr as *const u8;
    /// unsafe {
    ///     ptr = ptr.set_ptr_value(thin.add(8));
    ///     # assert_eq!(*(ptr as *const i32), 3);
    ///     println!("{:?}", &*ptr); // will print "3"
    /// }
    /// ```
    #[unstable(feature = "set_ptr_value", issue = "75091")]
    #[must_use = "returns a new pointer rather than modifying its argument"]
    #[inline]
    pub fn set_ptr_value(mut self, val: *const u8) -> Self {
        let thin = &mut self as *mut *const T as *mut *const u8;
        // SAFETY: In case of a thin pointer, this operations is identical
        // to a simple assignment. In case of a fat pointer, with the current
        // fat pointer layout implementation, the first field of such a
        // pointer is always the data pointer, which is likewise assigned.
        unsafe { *thin = val };
        self
    }

    /// Reads the value from `self` without moving it. This leaves the
    /// memory in `self` unchanged.
    ///
    /// See [`ptr::read`] for safety concerns and examples.
    ///
    /// [`ptr::read`]: crate::ptr::read()
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[rustc_const_unstable(feature = "const_ptr_read", issue = "80377")]
    #[inline]
    pub const unsafe fn read(self) -> T
    where
        T: Sized,
    {
        // SAFETY: the caller must uphold the safety contract for `read`.
        unsafe { read(self) }
    }

    /// Performs a volatile read of the value from `self` without moving it. This
    /// leaves the memory in `self` unchanged.
    ///
    /// Volatile operations are intended to act on I/O memory, and are guaranteed
    /// to not be elided or reordered by the compiler across other volatile
    /// operations.
    ///
    /// See [`ptr::read_volatile`] for safety concerns and examples.
    ///
    /// [`ptr::read_volatile`]: crate::ptr::read_volatile()
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub unsafe fn read_volatile(self) -> T
    where
        T: Sized,
    {
        // SAFETY: the caller must uphold the safety contract for `read_volatile`.
        unsafe { read_volatile(self) }
    }

    /// Reads the value from `self` without moving it. This leaves the
    /// memory in `self` unchanged.
    ///
    /// Unlike `read`, the pointer may be unaligned.
    ///
    /// See [`ptr::read_unaligned`] for safety concerns and examples.
    ///
    /// [`ptr::read_unaligned`]: crate::ptr::read_unaligned()
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[rustc_const_unstable(feature = "const_ptr_read", issue = "80377")]
    #[inline]
    pub const unsafe fn read_unaligned(self) -> T
    where
        T: Sized,
    {
        // SAFETY: the caller must uphold the safety contract for `read_unaligned`.
        unsafe { read_unaligned(self) }
    }

    /// Copies `count * size_of<T>` bytes from `self` to `dest`. The source
    /// and destination may overlap.
    ///
    /// NOTE: this has the *same* argument order as [`ptr::copy`].
    ///
    /// See [`ptr::copy`] for safety concerns and examples.
    ///
    /// [`ptr::copy`]: crate::ptr::copy()
    #[rustc_const_unstable(feature = "const_intrinsic_copy", issue = "80697")]
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub const unsafe fn copy_to(self, dest: *mut T, count: usize)
    where
        T: Sized,
    {
        // SAFETY: the caller must uphold the safety contract for `copy`.
        unsafe { copy(self, dest, count) }
    }

    /// Copies `count * size_of<T>` bytes from `self` to `dest`. The source
    /// and destination may *not* overlap.
    ///
    /// NOTE: this has the *same* argument order as [`ptr::copy_nonoverlapping`].
    ///
    /// See [`ptr::copy_nonoverlapping`] for safety concerns and examples.
    ///
    /// [`ptr::copy_nonoverlapping`]: crate::ptr::copy_nonoverlapping()
    #[rustc_const_unstable(feature = "const_intrinsic_copy", issue = "80697")]
    #[stable(feature = "pointer_methods", since = "1.26.0")]
    #[inline]
    pub const unsafe fn copy_to_nonoverlapping(self, dest: *mut T, count: usize)
    where
        T: Sized,
    {
        // SAFETY: the caller must uphold the safety contract for `copy_nonoverlapping`.
        unsafe { copy_nonoverlapping(self, dest, count) }
    }

    /// Computes the offset that needs to be applied to the pointer in order to make it aligned to
    /// `align`.
    ///
    /// If it is not possible to align the pointer, the implementation returns
    /// `usize::MAX`. It is permissible for the implementation to *always*
    /// return `usize::MAX`. Only your algorithm's performance can depend
    /// on getting a usable offset here, not its correctness.
    ///
    /// The offset is expressed in number of `T` elements, and not bytes. The value returned can be
    /// used with the `wrapping_add` method.
    ///
    /// There are no guarantees whatsoever that offsetting the pointer will not overflow or go
    /// beyond the allocation that the pointer points into. It is up to the caller to ensure that
    /// the returned offset is correct in all terms other than alignment.
    ///
    /// # Panics
    ///
    /// The function panics if `align` is not a power-of-two.
    ///
    /// # Examples
    ///
    /// Accessing adjacent `u8` as `u16`
    ///
    /// ```
    /// # fn foo(n: usize) {
    /// # use std::mem::align_of;
    /// # unsafe {
    /// let x = [5u8, 6u8, 7u8, 8u8, 9u8];
    /// let ptr = x.as_ptr().add(n) as *const u8;
    /// let offset = ptr.align_offset(align_of::<u16>());
    /// if offset < x.len() - n - 1 {
    ///     let u16_ptr = ptr.add(offset) as *const u16;
    ///     assert_ne!(*u16_ptr, 500);
    /// } else {
    ///     // while the pointer can be aligned via `offset`, it would point
    ///     // outside the allocation
    /// }
    /// # } }
    /// ```
    #[stable(feature = "align_offset", since = "1.36.0")]
    #[rustc_const_unstable(feature = "const_align_offset", issue = "90962")]
    pub const fn align_offset(self, align: usize) -> usize
    where
        T: Sized,
    {
        if !align.is_power_of_two() {
            panic!("align_offset: align is not a power-of-two");
        }

        fn rt_impl<T>(p: *const T, align: usize) -> usize {
            // SAFETY: `align` has been checked to be a power of 2 above
            unsafe { align_offset(p, align) }
        }

        const fn ctfe_impl<T>(_: *const T, _: usize) -> usize {
            usize::MAX
        }

        // SAFETY:
        // It is permisseble for `align_offset` to always return `usize::MAX`,
        // algorithm correctness can not depend on `align_offset` returning non-max values.
        //
        // As such the behaviour can't change after replacing `align_offset` with `usize::MAX`, only performance can.
        unsafe { intrinsics::const_eval_select((self, align), ctfe_impl, rt_impl) }
    }
}

#[lang = "const_slice_ptr"]
impl<T> *const [T] {
    /// Returns the length of a raw slice.
    ///
    /// The returned value is the number of **elements**, not the number of bytes.
    ///
    /// This function is safe, even when the raw slice cannot be cast to a slice
    /// reference because the pointer is null or unaligned.
    ///
    /// # Examples
    ///
    /// ```rust
    /// #![feature(slice_ptr_len)]
    ///
    /// use std::ptr;
    ///
    /// let slice: *const [i8] = ptr::slice_from_raw_parts(ptr::null(), 3);
    /// assert_eq!(slice.len(), 3);
    /// ```
    #[inline]
    #[unstable(feature = "slice_ptr_len", issue = "71146")]
    #[rustc_const_unstable(feature = "const_slice_ptr_len", issue = "71146")]
    pub const fn len(self) -> usize {
        metadata(self)
    }

    /// Returns a raw pointer to the slice's buffer.
    ///
    /// This is equivalent to casting `self` to `*const T`, but more type-safe.
    ///
    /// # Examples
    ///
    /// ```rust
    /// #![feature(slice_ptr_get)]
    /// use std::ptr;
    ///
    /// let slice: *const [i8] = ptr::slice_from_raw_parts(ptr::null(), 3);
    /// assert_eq!(slice.as_ptr(), ptr::null());
    /// ```
    #[inline]
    #[unstable(feature = "slice_ptr_get", issue = "74265")]
    #[rustc_const_unstable(feature = "slice_ptr_get", issue = "74265")]
    pub const fn as_ptr(self) -> *const T {
        self as *const T
    }

    /// Returns a raw pointer to an element or subslice, without doing bounds
    /// checking.
    ///
    /// Calling this method with an out-of-bounds index or when `self` is not dereferenceable
    /// is *[undefined behavior]* even if the resulting pointer is not used.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    /// # Examples
    ///
    /// ```
    /// #![feature(slice_ptr_get)]
    ///
    /// let x = &[1, 2, 4] as *const [i32];
    ///
    /// unsafe {
    ///     assert_eq!(x.get_unchecked(1), x.as_ptr().add(1));
    /// }
    /// ```
    #[unstable(feature = "slice_ptr_get", issue = "74265")]
    #[rustc_const_unstable(feature = "const_slice_index", issue = "none")]
    #[inline]
    pub const unsafe fn get_unchecked<I>(self, index: I) -> *const I::Output
    where
        I: ~const SliceIndex<[T]>,
    {
        // SAFETY: the caller ensures that `self` is dereferenceable and `index` in-bounds.
        unsafe { index.get_unchecked(self) }
    }

    /// Returns `None` if the pointer is null, or else returns a shared slice to
    /// the value wrapped in `Some`. In contrast to [`as_ref`], this does not require
    /// that the value has to be initialized.
    ///
    /// [`as_ref`]: #method.as_ref
    ///
    /// # Safety
    ///
    /// When calling this method, you have to ensure that *either* the pointer is null *or*
    /// all of the following is true:
    ///
    /// * The pointer must be [valid] for reads for `ptr.len() * mem::size_of::<T>()` many bytes,
    ///   and it must be properly aligned. This means in particular:
    ///
    ///     * The entire memory range of this slice must be contained within a single [allocated object]!
    ///       Slices can never span across multiple allocated objects.
    ///
    ///     * The pointer must be aligned even for zero-length slices. One
    ///       reason for this is that enum layout optimizations may rely on references
    ///       (including slices of any length) being aligned and non-null to distinguish
    ///       them from other data. You can obtain a pointer that is usable as `data`
    ///       for zero-length slices using [`NonNull::dangling()`].
    ///
    /// * The total size `ptr.len() * mem::size_of::<T>()` of the slice must be no larger than `isize::MAX`.
    ///   See the safety documentation of [`pointer::offset`].
    ///
    /// * You must enforce Rust's aliasing rules, since the returned lifetime `'a` is
    ///   arbitrarily chosen and does not necessarily reflect the actual lifetime of the data.
    ///   In particular, for the duration of this lifetime, the memory the pointer points to must
    ///   not get mutated (except inside `UnsafeCell`).
    ///
    /// This applies even if the result of this method is unused!
    ///
    /// See also [`slice::from_raw_parts`][].
    ///
    /// [valid]: crate::ptr#safety
    /// [allocated object]: crate::ptr#allocated-object
    #[inline]
    #[unstable(feature = "ptr_as_uninit", issue = "75402")]
    #[rustc_const_unstable(feature = "const_ptr_as_ref", issue = "91822")]
    pub const unsafe fn as_uninit_slice<'a>(self) -> Option<&'a [MaybeUninit<T>]> {
        if self.is_null() {
            None
        } else {
            // SAFETY: the caller must uphold the safety contract for `as_uninit_slice`.
            Some(unsafe { slice::from_raw_parts(self as *const MaybeUninit<T>, self.len()) })
        }
    }
}

// Equality for pointers
#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> PartialEq for *const T {
    #[inline]
    fn eq(&self, other: &*const T) -> bool {
        *self == *other
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> Eq for *const T {}

// Comparison for pointers
#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> Ord for *const T {
    #[inline]
    fn cmp(&self, other: &*const T) -> Ordering {
        if self < other {
            Less
        } else if self == other {
            Equal
        } else {
            Greater
        }
    }
}

#[stable(feature = "rust1", since = "1.0.0")]
impl<T: ?Sized> PartialOrd for *const T {
    #[inline]
    fn partial_cmp(&self, other: &*const T) -> Option<Ordering> {
        Some(self.cmp(other))
    }

    #[inline]
    fn lt(&self, other: &*const T) -> bool {
        *self < *other
    }

    #[inline]
    fn le(&self, other: &*const T) -> bool {
        *self <= *other
    }

    #[inline]
    fn gt(&self, other: &*const T) -> bool {
        *self > *other
    }

    #[inline]
    fn ge(&self, other: &*const T) -> bool {
        *self >= *other
    }
}
