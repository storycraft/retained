#[repr(transparent)]
pub struct Ptr<T> {
    ptr: *mut T,
}

impl<T> Ptr<T> {
    pub fn new(ptr: *mut T) -> Ptr<T> {
        Ptr { ptr }
    }

    #[inline(always)]
    pub const fn cast<U>(self) -> Ptr<U> {
        Ptr {
            ptr: self.ptr.cast(),
        }
    }

    #[inline(always)]
    pub unsafe fn as_mut(&self) -> &mut T {
        &mut *self.ptr
    }
}
