use std::marker::PhantomData;

#[repr(transparent)]
pub struct Ptr<'a, T> {
    ptr: *mut T,
    _ph: PhantomData<&'a mut T>,
}

impl<'a, T> Ptr<'a, T> {
    #[inline(always)]
    pub fn new(ptr: &'a mut T) -> Ptr<'a, T> {
        Ptr {
            ptr,
            _ph: PhantomData,
        }
    }

    #[inline(always)]
    pub unsafe fn byte_add(self, count: usize) -> Self {
        Ptr {
            ptr: self.ptr.byte_add(count),
            _ph: PhantomData,
        }
    }

    #[inline(always)]
    pub fn cast<U>(self) -> Ptr<'a, U> {
        Ptr {
            ptr: self.ptr.cast(),
            _ph: PhantomData,
        }
    }

    #[inline(always)]
    pub unsafe fn as_mut(self) -> &'a mut T {
        &mut *self.ptr
    }
}
