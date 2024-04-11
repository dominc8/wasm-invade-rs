use std::mem;

pub fn static_alloc<T: Sized>() -> &'static mut T {
    unsafe { STATIC_ALLOCATOR.alloc_obj() }
}

pub fn static_dealloc<T: Sized>(obj: &mut T) {
    unsafe { STATIC_ALLOCATOR.dealloc_obj(obj) }
}

fn static_alloc_mult<T: Sized>(n: usize) -> *mut T {
    unsafe { STATIC_ALLOCATOR.alloc_obj_mult(n) }
}

fn static_dealloc_mult<T: Sized>(obj_ptr: *mut T, n: usize) {
    unsafe { STATIC_ALLOCATOR.dealloc_obj_mult(obj_ptr, n) }
}

const MEM_SIZE: usize = 2 << 10;
struct StaticAllocator {
    memory: [u64; MEM_SIZE],
    used_bitmap: [u64; MEM_SIZE / 64],
}

static mut STATIC_ALLOCATOR: StaticAllocator = StaticAllocator{ memory: [0; MEM_SIZE], used_bitmap: [0; MEM_SIZE / 64] };

impl StaticAllocator {
    unsafe fn alloc_obj<T: Sized>(&mut self) -> &mut T {
        let (size, align) = Self::round_bitmap_size_align::<T>();
        for (idx, bm) in self.used_bitmap.iter_mut().enumerate() {
            if let Ok(bit0) = Self::find_free_bitrange(*bm, size, align) {
                let memory_offset = idx * 64 + bit0;
                let obj_ptr = self.memory.as_mut_ptr().wrapping_add(memory_offset);
                *bm |= ((1 << size) - 1) << bit0;
                return &mut*(obj_ptr as *mut T)
            }
        }
        // this never happens but Rust doesn't know it, 
        // (it may happen in general case, but not in this project)
        loop {}
        //let obj_ptr = self.memory.as_mut_ptr();
        //return &mut*(obj_ptr as *mut T)
    }

    unsafe fn dealloc_obj<T: Sized>(&mut self, obj: &mut T) {
        let (size, _) = Self::round_bitmap_size_align::<T>();
        let obj_ptr = (obj as *mut T) as *mut u64;
        let memory_offset = obj_ptr.offset_from(self.memory.as_ptr()) as usize;
        let idx = memory_offset / 64;
        let bit0 = memory_offset % 64;
        if let Some(bm) = self.used_bitmap.get_mut(idx) {
            let freeing_bm = !(((1 << size) - 1) << bit0);
            *bm &= freeing_bm;
        }
    }

    unsafe fn alloc_obj_mult<T: Sized>(&mut self, n: usize) -> *mut T {
        let (size, align) = Self::round_bitmap_size_align_mult::<T>(n);

        for (idx, bm) in self.used_bitmap.iter_mut().enumerate() {
            if let Ok(bit0) = Self::find_free_bitrange(*bm, size, align) {
                let memory_offset = idx * 64 + bit0;
                let mem_ptr = self.memory.as_mut_ptr().wrapping_add(memory_offset);
                *bm |= ((1 << size) - 1) << bit0;
                let obj_ptr = mem_ptr as *mut T;
                return obj_ptr
            }
        }
        // this never happens but Rust doesn't know it, 
        // (it may happen in general case, but not in this project)
        loop {}
        //let obj_ptr = self.memory.as_mut_ptr();
        //return &mut*(obj_ptr as *mut T)
    }

    unsafe fn dealloc_obj_mult<T: Sized>(&mut self, obj_ptr: *mut T, n: usize) {
        let (size, _) = Self::round_bitmap_size_align_mult::<T>(n);
        let mem_ptr = obj_ptr as *mut u64;
        let memory_offset = mem_ptr.offset_from(self.memory.as_ptr()) as usize;
        let idx = memory_offset / 64;
        let bit0 = memory_offset % 64;
        if let Some(bm) = self.used_bitmap.get_mut(idx) {
            let freeing_bm = !(((1 << size) - 1) << bit0);
            *bm &= freeing_bm;
        }
        else {
            loop {}
        }
    }

    fn find_free_bitrange(bm: u64, size: usize, align: usize) -> Result<usize, ()> {
        let mut first_zero = Self::find_next_zero(bm, 0);
        while first_zero < 64 {
            let next_one = Self::find_next_one(bm, first_zero);
            let aligned_first_zero = ((first_zero + align - 1) / align ) * align;
            if aligned_first_zero < next_one && next_one - aligned_first_zero >= size {
                return Ok(aligned_first_zero);
            }
            first_zero = Self::find_next_zero(bm, next_one);
        }
        Err(())
    }

    fn find_next_zero(bm: u64, bit_start: usize) -> usize {
        for i in bit_start..64 {
            if (bm & 1 << i) == 0 {
                return i;
            }
        }
        return 64;
    }

    fn find_next_one(bm: u64, bit_start: usize) -> usize {
        for i in bit_start..64 {
            if (bm & 1 << i) == (1 << i) {
                return i;
            }
        }
        return 64;
    }

    const fn round_bitmap_size_align<T: Sized>() -> (usize, usize) {
        let unit_size = mem::size_of::<u64>();
        let t_size = mem::size_of::<T>();
        let t_align = mem::size_of::<T>();
        let size = (t_size + unit_size - 1) / unit_size;
        let align = (t_align + unit_size - 1) / unit_size;
        (size, align)
    }

    fn round_bitmap_size_align_mult<T: Sized>(n: usize) -> (usize, usize) {
        let unit_size = mem::size_of::<u64>();
        let t_size = mem::size_of::<T>();
        let t_align = mem::size_of::<T>();
        let size = (n * t_size + unit_size - 1) / unit_size;
        let align = (t_align + unit_size - 1) / unit_size;
        (size, align)
    }
}

pub struct SVector<T: Sized> {
    elements: *mut T,
    size: usize,
    capacity: usize,
}

impl<T> SVector<T> {
    pub fn new(n: usize) -> SVector<T> {
        let obj_ptr = static_alloc_mult(n);
        SVector { elements: obj_ptr, size: 0, capacity: n }
    }

    pub fn get(&self, idx: usize) -> Option<&T> {
        if idx < self.size {
            unsafe { self.elements.offset(idx as isize).as_ref() }
        } else {
            None
        }
    }

    pub fn get_mut(&self, idx: usize) -> Option<&mut T> {
        if idx < self.size {
            unsafe { self.elements.offset(idx as isize).as_mut() }
        } else {
            None
        }
    }

    pub fn remove(&mut self, idx: usize) {
        if idx < self.size {
            let new_size = self.size - 1;
            self.size = new_size;

            let n_to_copy = new_size - idx;
            if n_to_copy == 0 {
                return
            }

            unsafe {
                let dst_ptr = self.elements.offset(idx as isize);
                let src_ptr = dst_ptr.offset(1);
                src_ptr.copy_to(dst_ptr, n_to_copy);
            }
        }
    }

    pub fn pop(&mut self) {
        if self.size > 0 {
            self.remove(self.size - 1);
        }
    }

    pub fn push_back(&mut self, val: T) {
        if self.size < self.capacity {
            let idx = self.size;
            self.size += 1;

            if let Some(x) = self.get_mut(idx) {
                *x = val;
            }
        } else {
            loop {}
        }
    }
}

impl<T> Drop for SVector<T> {
    fn drop(&mut self) {
        static_dealloc_mult(self.elements, self.capacity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_fn() {
        assert_eq!(StaticAllocator::round_bitmap_size_align::<u8>(), (1, 1));
        assert_eq!(StaticAllocator::round_bitmap_size_align::<u16>(), (1, 1));
        assert_eq!(StaticAllocator::round_bitmap_size_align::<u32>(), (1, 1));
        assert_eq!(StaticAllocator::round_bitmap_size_align::<u64>(), (1, 1));
        assert_eq!(StaticAllocator::round_bitmap_size_align::<u128>(), (2, 2));
    }

    #[test]
    fn test_find_next_zero() {
        assert_eq!(StaticAllocator::find_next_zero(0b0, 0), 0);
        assert_eq!(StaticAllocator::find_next_zero(!0b0, 0), 64);

        assert_eq!(StaticAllocator::find_next_zero(0b1, 0), 1);
        assert_eq!(StaticAllocator::find_next_zero(0b1, 1), 1);

        assert_eq!(StaticAllocator::find_next_zero(0b11101111, 0), 4);
        assert_eq!(StaticAllocator::find_next_zero(0b11101111, 5), 8);

    }

    #[test]
    fn test_find_next_one() {
        assert_eq!(StaticAllocator::find_next_one(0b0, 0), 64);
        assert_eq!(StaticAllocator::find_next_one(!0b0, 0), 0);

        assert_eq!(StaticAllocator::find_next_one(0b1, 0), 0);
        assert_eq!(StaticAllocator::find_next_one(0b1, 1), 64);

        assert_eq!(StaticAllocator::find_next_one(0b11100110, 0), 1);
        assert_eq!(StaticAllocator::find_next_one(0b11100110, 3), 5);
    }

    #[test]
    fn test_find_free_bitrange() {
        assert_eq!(StaticAllocator::find_free_bitrange(0b0, 1, 1), Ok(0));
        assert_eq!(StaticAllocator::find_free_bitrange(!0b0, 1, 1), Err(()));

        assert_eq!(StaticAllocator::find_free_bitrange(0b1, 1, 1), Ok(1));
        assert_eq!(StaticAllocator::find_free_bitrange(0b111111, 1, 1), Ok(6));

        assert_eq!(StaticAllocator::find_free_bitrange(0b111100, 2, 1), Ok(0));
        assert_eq!(StaticAllocator::find_free_bitrange(0b111100, 2, 2), Ok(0));
        assert_eq!(StaticAllocator::find_free_bitrange(0b111001, 2, 1), Ok(1));
        assert_eq!(StaticAllocator::find_free_bitrange(0b111001, 2, 2), Ok(6));

        assert_eq!(StaticAllocator::find_free_bitrange(0xAA_AA_AA_AA_AA_AA_AA_AA, 2, 2), Err(()));
    }

    #[test]
    fn alloc_dealloc_u8() {
        let obj_u8 = static_alloc::<u8>();
        let obj_ptr = (obj_u8 as *mut u8) as *mut u64;
        unsafe { assert_eq!(obj_ptr, STATIC_ALLOCATOR.memory.as_mut_ptr()) }
        unsafe { assert_eq!(0b1, STATIC_ALLOCATOR.used_bitmap[0]) }

        static_dealloc(obj_u8);
        unsafe { assert_eq!(0b0, STATIC_ALLOCATOR.used_bitmap[0]) }
    }

    #[test]
    fn alloc_dealloc_2_u64s() {
        let obj0_u64 = static_alloc::<u64>();
        let obj0_ptr = obj0_u64 as *mut u64;
        unsafe { assert_eq!(obj0_ptr, STATIC_ALLOCATOR.memory.as_mut_ptr()) }
        unsafe { assert_eq!(0b1, STATIC_ALLOCATOR.used_bitmap[0]) }

        let obj1_u64 = static_alloc::<u64>();
        let obj1_ptr = obj1_u64 as *mut u64;
        unsafe { assert_eq!(obj1_ptr, STATIC_ALLOCATOR.memory.as_mut_ptr().wrapping_add(1)) }
        unsafe { assert_eq!(0b11, STATIC_ALLOCATOR.used_bitmap[0]) }

        static_dealloc(obj0_u64);
        unsafe { assert_eq!(0b10, STATIC_ALLOCATOR.used_bitmap[0]) }

        static_dealloc(obj1_u64);
        unsafe { assert_eq!(0b0, STATIC_ALLOCATOR.used_bitmap[0]) }
    }

    #[test]
    fn alloc_dealloc_u8_arrays() {
        let arr0_u8 = static_alloc_mult::<u8>(7);
        let arr0_ptr = (arr0_u8 as *mut u8) as *mut u64;
        unsafe { assert_eq!(arr0_ptr, STATIC_ALLOCATOR.memory.as_mut_ptr()) }
        unsafe { assert_eq!(0b1, STATIC_ALLOCATOR.used_bitmap[0]) }

        let arr1_u8 = static_alloc_mult::<u8>(8);
        let arr1_ptr = (arr0_u8 as *mut u8) as *mut u64;
        unsafe { assert_eq!(arr1_ptr, STATIC_ALLOCATOR.memory.as_mut_ptr()) }
        unsafe { assert_eq!(0b11, STATIC_ALLOCATOR.used_bitmap[0]) }

        let arr2_u8 = static_alloc_mult::<u8>(9);
        let arr2_ptr = (arr0_u8 as *mut u8) as *mut u64;
        unsafe { assert_eq!(arr2_ptr, STATIC_ALLOCATOR.memory.as_mut_ptr()) }
        unsafe { assert_eq!(0b1111, STATIC_ALLOCATOR.used_bitmap[0]) }

        static_dealloc_mult(arr1_u8, 8);
        unsafe { assert_eq!(0b1101, STATIC_ALLOCATOR.used_bitmap[0]) }
        static_dealloc_mult(arr0_u8, 7);
        unsafe { assert_eq!(0b1100, STATIC_ALLOCATOR.used_bitmap[0]) }
        static_dealloc_mult(arr2_u8, 9);
        unsafe { assert_eq!(0b0, STATIC_ALLOCATOR.used_bitmap[0]) }
    }
}
