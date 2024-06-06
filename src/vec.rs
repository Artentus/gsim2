use std::fmt;
use std::mem::{self, ManuallyDrop, MaybeUninit};
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::slice;

#[repr(C)]
struct Inline<T, const CAP: usize> {
    len: i32,
    data: [MaybeUninit<T>; CAP],
}

impl<T, const CAP: usize> Inline<T, CAP> {
    const INIT: MaybeUninit<T> = MaybeUninit::uninit();

    #[inline]
    const fn new() -> Self {
        assert!(CAP <= (i32::MAX as usize), "inline capacity overflow");

        Self {
            len: 0,
            data: [Self::INIT; CAP],
        }
    }

    #[inline]
    fn from_array<const N: usize>(array: [T; N]) -> Self {
        assert!(N <= CAP);

        let mut this = Self::new();
        this.len = N as i32;

        for (dst, src) in this.data.iter_mut().zip(array) {
            dst.write(src);
        }

        this
    }

    #[inline]
    const fn len(&self) -> usize {
        assert!(self.len >= 0);
        self.len as usize
    }

    #[inline]
    fn as_slice(&self) -> &[T] {
        let len = self.len();
        unsafe {
            // SAFETY: `MaybeUninit<T>` has the same layout as `T`
            mem::transmute(&self.data[..len])
        }
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len();
        unsafe {
            // SAFETY: `MaybeUninit<T>` has the same layout as `T`
            mem::transmute(&mut self.data[..len])
        }
    }

    #[inline]
    fn push(&mut self, value: T) {
        let len = self.len();
        self.data[len].write(value);
        self.len += 1;
    }

    #[inline]
    fn pop(&mut self) -> Option<T> {
        if self.len() > 0 {
            self.len -= 1;
            let len = self.len();
            unsafe { Some(self.data[len].assume_init_read()) }
        } else {
            None
        }
    }
}

impl<T: Clone, const CAP: usize> Inline<T, CAP> {
    #[inline]
    fn from_slice(slice: &[T]) -> Self {
        assert!(slice.len() <= CAP);

        let mut this = Self::new();
        this.len = slice.len() as i32;

        for (dst, src) in this.data.iter_mut().zip(slice) {
            dst.write(src.clone());
        }

        this
    }

    #[inline]
    fn repeat(value: T, count: usize) -> Self {
        assert!(count <= CAP);

        let mut this = Self::new();
        this.len = count as i32;

        if count > 0 {
            let mut iter = this.data.iter_mut();

            for dst in iter.by_ref().take(count - 1) {
                dst.write(value.clone());
            }

            let final_dst = iter.next().unwrap();
            final_dst.write(value);
        }

        this
    }
}

impl<T: Clone, const CAP: usize> Clone for Inline<T, CAP> {
    #[inline]
    fn clone(&self) -> Self {
        let slice = self.as_slice();
        Self::from_slice(slice)
    }
}

impl<T, const CAP: usize> Drop for Inline<T, CAP> {
    #[inline]
    fn drop(&mut self) {
        let len = self.len();
        for value in &mut self.data[..len] {
            unsafe { value.assume_init_drop() };
        }
    }
}

#[repr(C)]
struct Heap<T> {
    len: i32,
    cap: u32,
    data: NonNull<T>,
}

unsafe impl<T: Send> Send for Heap<T> {}
unsafe impl<T: Sync> Sync for Heap<T> {}

impl<T> Heap<T> {
    #[inline]
    fn from_vec(mut vec: Vec<T>) -> Self {
        let len = !i32::try_from(vec.len()).expect("length overflow");
        let cap = vec.capacity().try_into().expect("capacity overflow");
        let data = NonNull::new(vec.as_mut_ptr()).expect("invalid pointer");
        mem::forget(vec);

        Self { len, cap, data }
    }

    /// SAFETY: all elements in `inline` must be initialized
    #[inline]
    unsafe fn from_inline<const INLINE_CAP: usize>(
        inline: &mut Inline<T, INLINE_CAP>,
        value: T,
    ) -> Self {
        assert_eq!(inline.len(), INLINE_CAP);

        let mut vec = Vec::new();
        vec.reserve(INLINE_CAP + 1);
        for inline_value in &inline.data {
            unsafe { vec.push(inline_value.assume_init_read()) };
        }
        inline.len = 0; // data was moved
        vec.push(value);

        Self::from_vec(vec)
    }

    /// SAFETY: all elements in `inline` must be initialized
    #[inline]
    unsafe fn from_inline_and_remaining<const INLINE_CAP: usize>(
        inline: &mut Inline<T, INLINE_CAP>,
        value: T,
        remaining: impl Iterator<Item = T>,
    ) -> Self {
        assert_eq!(inline.len(), INLINE_CAP);

        let mut vec = Vec::new();
        vec.reserve(INLINE_CAP + 1);
        for inline_value in &inline.data {
            unsafe { vec.push(inline_value.assume_init_read()) };
        }
        inline.len = 0; // data was moved
        vec.push(value);
        vec.extend(remaining);

        Self::from_vec(vec)
    }

    #[inline]
    fn from_array<const N: usize>(array: [T; N]) -> Self {
        let vec = <[_]>::into_vec(Box::new(array));
        Self::from_vec(vec)
    }

    #[inline]
    const fn len(&self) -> usize {
        assert!(self.len < 0);
        (!self.len) as usize
    }

    #[inline]
    fn as_vec(&mut self) -> Vec<T> {
        let len = self.len();
        let cap = self.cap as usize;
        let data = self.data.as_ptr();
        unsafe { Vec::from_raw_parts(data, len, cap) }
    }

    #[inline]
    fn as_slice(&self) -> &[T] {
        let len = self.len();
        let data = self.data.as_ptr().cast_const();
        unsafe { slice::from_raw_parts(data, len) }
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [T] {
        let len = self.len();
        let data = self.data.as_ptr();
        unsafe { slice::from_raw_parts_mut(data, len) }
    }

    #[inline]
    fn push(&mut self, value: T) {
        let mut vec = self.as_vec();
        vec.push(value);
        *self = Self::from_vec(vec);
    }

    #[inline]
    fn pop(&mut self) -> Option<T> {
        let mut vec = self.as_vec();
        let value = vec.pop();
        *self = Self::from_vec(vec);
        value
    }
}

impl<T: Clone> Heap<T> {
    #[inline]
    fn from_slice(slice: &[T]) -> Self {
        let vec = slice.to_vec();
        Self::from_vec(vec)
    }

    #[inline]
    fn repeat(value: T, count: usize) -> Self {
        let vec = vec![value; count];
        Self::from_vec(vec)
    }
}

impl<T: Clone> Clone for Heap<T> {
    #[inline]
    fn clone(&self) -> Self {
        let slice = self.as_slice();
        Self::from_slice(slice)
    }
}

impl<T> Drop for Heap<T> {
    #[inline]
    fn drop(&mut self) {
        let _ = self.as_vec();
    }
}

#[repr(C)]
pub union SmallVec<T, const INLINE_CAP: usize> {
    /// SAFETY: __never__ write to this field to not invalidate the data
    len: i32,
    inline: ManuallyDrop<Inline<T, INLINE_CAP>>,
    heap: ManuallyDrop<Heap<T>>,
}

impl<T, const INLINE_CAP: usize> SmallVec<T, INLINE_CAP> {
    #[inline]
    pub const fn new() -> Self {
        Self {
            inline: ManuallyDrop::new(Inline::new()),
        }
    }

    pub fn from_array<const N: usize>(array: [T; N]) -> Self {
        if N <= INLINE_CAP {
            Self {
                inline: ManuallyDrop::new(Inline::from_array(array)),
            }
        } else {
            Self {
                heap: ManuallyDrop::new(Heap::from_array(array)),
            }
        }
    }

    #[inline]
    pub const fn is_inline(&self) -> bool {
        unsafe {
            // SAFETY: len is always initialized because the field exists at the same location in all variants
            self.len > 0
        }
    }

    #[inline]
    pub const fn len(&self) -> usize {
        let len = unsafe {
            // SAFETY: len is always initialized because the field exists at the same location in all variants
            self.len
        };

        if len < 0 {
            (!len) as usize
        } else {
            len as usize
        }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        if self.is_inline() {
            INLINE_CAP
        } else {
            unsafe { self.heap.cap as usize }
        }
    }

    pub fn push(&mut self, value: T) {
        if self.is_inline() {
            let len = self.len();
            let inline = unsafe { &mut self.inline };

            if len == INLINE_CAP {
                let new_heap = unsafe {
                    // SAFETY: when len == INLINE_CAP, all elements in the array are initialized
                    Heap::from_inline(inline, value)
                };

                self.heap = ManuallyDrop::new(new_heap);
            } else {
                inline.push(value);
            }
        } else {
            unsafe { self.heap.push(value) };
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.is_inline() {
            unsafe { self.inline.pop() }
        } else {
            unsafe { self.heap.pop() }
        }
    }

    pub fn as_slice(&self) -> &[T] {
        if self.is_inline() {
            unsafe { self.inline.as_slice() }
        } else {
            unsafe { self.heap.as_slice() }
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        if self.is_inline() {
            unsafe { self.inline.as_mut_slice() }
        } else {
            unsafe { self.heap.as_mut_slice() }
        }
    }
}

impl<T, const INLINE_CAP: usize> Deref for SmallVec<T, INLINE_CAP> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const INLINE_CAP: usize> DerefMut for SmallVec<T, INLINE_CAP> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, const INLINE_CAP: usize> AsRef<[T]> for SmallVec<T, INLINE_CAP> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const INLINE_CAP: usize> AsMut<[T]> for SmallVec<T, INLINE_CAP> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: Clone, const INLINE_CAP: usize> SmallVec<T, INLINE_CAP> {
    pub fn from_slice(slice: &[T]) -> Self {
        if slice.len() <= INLINE_CAP {
            Self {
                inline: ManuallyDrop::new(Inline::from_slice(slice)),
            }
        } else {
            Self {
                heap: ManuallyDrop::new(Heap::from_slice(slice)),
            }
        }
    }

    pub fn repeat(value: T, count: usize) -> Self {
        if count <= INLINE_CAP {
            Self {
                inline: ManuallyDrop::new(Inline::repeat(value, count)),
            }
        } else {
            Self {
                heap: ManuallyDrop::new(Heap::repeat(value, count)),
            }
        }
    }
}

impl<T: Clone, const INLINE_CAP: usize> From<&[T]> for SmallVec<T, INLINE_CAP> {
    #[inline]
    fn from(slice: &[T]) -> Self {
        Self::from_slice(slice)
    }
}

impl<T, const N: usize, const INLINE_CAP: usize> From<[T; N]> for SmallVec<T, INLINE_CAP> {
    #[inline]
    fn from(array: [T; N]) -> Self {
        Self::from_array(array)
    }
}

impl<T, const INLINE_CAP: usize> FromIterator<T> for SmallVec<T, INLINE_CAP> {
    fn from_iter<Iter: IntoIterator<Item = T>>(iter: Iter) -> Self {
        let mut iter = iter.into_iter();
        let (min_size, _) = iter.size_hint();

        if min_size > INLINE_CAP {
            let vec = Vec::from_iter(iter);
            Self {
                heap: ManuallyDrop::new(Heap::from_vec(vec)),
            }
        } else {
            let mut inline = Inline::new();
            for value in iter.by_ref().take(INLINE_CAP) {
                inline.push(value);
            }

            if let Some(value) = iter.next() {
                let heap = unsafe {
                    // SAFETY: execution only reaches here if all elements in `inline` were initialized
                    Heap::from_inline_and_remaining(&mut inline, value, iter)
                };
                mem::forget(inline);

                Self {
                    heap: ManuallyDrop::new(heap),
                }
            } else {
                Self {
                    inline: ManuallyDrop::new(inline),
                }
            }
        }
    }
}

impl<T: Clone, const INLINE_CAP: usize> Clone for SmallVec<T, INLINE_CAP> {
    fn clone(&self) -> Self {
        if self.is_inline() {
            unsafe {
                Self {
                    inline: self.inline.clone(),
                }
            }
        } else {
            unsafe {
                Self {
                    heap: self.heap.clone(),
                }
            }
        }
    }
}

impl<T: fmt::Debug, const INLINE_CAP: usize> fmt::Debug for SmallVec<T, INLINE_CAP> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_slice(), f)
    }
}

impl<T, const INLINE_CAP: usize> Drop for SmallVec<T, INLINE_CAP> {
    fn drop(&mut self) {
        if self.is_inline() {
            unsafe {
                ManuallyDrop::drop(&mut self.inline);
            }
        } else {
            unsafe {
                ManuallyDrop::drop(&mut self.heap);
            }
        }
    }
}

/// `small_vec![]`
macro_rules! small_vec {
    () => {
        $crate::vec::SmallVec::new()
    };
    ($value:expr; $count:expr) => {
        $crate::vec::SmallVec::repeat($value, $count)
    };
    ($($value:expr),+ $(,)?) => {
        $crate::vec::SmallVec::from_array([$($value),+])
    }
}

pub(crate) use small_vec;
