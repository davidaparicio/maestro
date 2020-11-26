/*
 * This file handles allocations of chunks of kernel memory.
 *
 * TODO: More documentation
 */

use core::ffi::c_void;
use core::mem::MaybeUninit;
use crate::memory::PAGE_SIZE;
use crate::memory::buddy;
use crate::util::data_struct::LinkedList;
use crate::util;

/*
 * Type representing chunks' flags.
 */
type ChunkFlags = u8;

/* Chunk flag indicating that the chunk is being used */
const CHUNK_FLAG_USED: ChunkFlags = 0b1;

/*
 * The minimum amount of bytes required to create a free chunk.
 */
const FREE_CHUNK_MIN: usize = 8;

/*
 * The size of the smallest free list bin.
 */
const FREE_LIST_SMALLEST_SIZE: usize = FREE_CHUNK_MIN;

/*
 * The number of free list bins.
 */
const FREE_LIST_BINS: usize = 8;

/*
 * A chunk of allocated or free memory stored in linked lists.
 */
struct Chunk {
	/* The linked list storing the chunks */
	list: LinkedList,
	/* The chunk's flags */
	flags: u8,
	/* The size of the chunk's memory in bytes */
	size: usize,
}

/*
 * Structure representing a frame of memory allocated using the buddy allocator, storing memory
 * chunks.
 */
struct Block {
	/* The linked list storing the blocks */
	list: LinkedList,
	/* The order of the frame for the buddy allocator */
	order: buddy::FrameOrder,
	/* The first chunk of the block */
	first_chunk: Chunk,
}

impl Chunk {
	/*
	 * Creates a new free chunk with the given size `size` in bytes.
	 */
	fn new_free(size: usize) -> Self {
		Self {
			list: LinkedList::new_single(),
			flags: 0,
			size: size,
		}
	}

	/*
	 * Tells the whether the chunk is free.
	 */
	fn is_used(&self) -> bool {
		(self.flags & CHUNK_FLAG_USED) != 0
	}

	/*
	 * Tells whether the chunk can be split for the given size `size`.
	 */
	fn can_split(&self, size: usize) -> bool {
		self.size >= size + core::mem::size_of::<Chunk>() + FREE_CHUNK_MIN
	}

	/*
	 * Splits the chunk with the given size `size` if necessary and marks it as used. The function
	 * might create a new chunk next to the current.
	 */
	fn split(&mut self, size: usize) {
		if self.can_split(size) {
			let next_off = (self as *mut Self as usize) + core::mem::size_of::<Chunk>() + size;
			unsafe {
				let next = &mut *(next_off as *mut Self);
				util::bzero(next as *mut Self as _, core::mem::size_of::<Chunk>());
				next.flags = 0;
				next.size = self.size - (size + core::mem::size_of::<Chunk>());
			}
		}

		self.flags |= CHUNK_FLAG_USED;
	}

	/*
	 * Marks the chunk as free and tries to coalesce it with adjacent chunks if they are free.
	 */
	fn coalesce(&mut self) {
		self.flags &= CHUNK_FLAG_USED;

		if let Some(next) = self.list.get_next() {
			let n = unsafe {
				&*crate::linked_list_get!(next as *mut LinkedList, *const Chunk, list)
			};

			if !n.is_used() {
				self.size += core::mem::size_of::<Chunk>() + n.size;
				next.unlink();
			}
		}

		if let Some(prev) = self.list.get_prev() {
			let p = unsafe {
				&mut *crate::linked_list_get!(prev as *mut LinkedList, *mut Chunk, list)
			};

			if !p.is_used() {
				p.coalesce();
			}
		}
	}

	/*
	 * Tries to resize the chunk, adding `delta` bytes. A negative number of bytes results in chunk
	 * shrinking. Returns `true` if possible, or `false` if not. If the chunk cannot be expanded,
	 * the function does nothing. Expansion might reduce/move/remove the next chunk if it is free.
	 * If `delta` is zero, the function returns `false`.
	 */
	fn resize(&mut self, delta: isize) -> bool {
		if delta == 0 {
			return true;
		}

		let mut valid = false;

		if delta > 0 {
			if let Some(next) = self.list.get_next() {
				let n = unsafe {
					&*crate::linked_list_get!(next as *mut LinkedList, *const Chunk, list)
				};

				if n.is_used() {
					return false;
				}

				let available_size = core::mem::size_of::<Chunk>() + n.size;
				if available_size < delta as usize {
					return false;
				}

				let next_min_size = core::mem::size_of::<Chunk>() + FREE_CHUNK_MIN;
				if available_size - delta as usize >= next_min_size {
					// TODO Move next chunk
				} else {
					next.unlink();
				}

				valid = true;
			}
		}

		if delta < 0 {
			if self.size <= (-delta) as usize {
				return false;
			}

			if let Some(next) = self.list.get_next() {
				let n = unsafe {
					&*crate::linked_list_get!(next as *mut LinkedList, *const Chunk, list)
				};

				if !n.is_used() {
					// TODO Move next chunk
				}
			}

			valid = true;
		}

		if valid {
			if delta >= 0 {
				self.size += delta as usize;
			} else {
				self.size -= delta.abs() as usize;
			}
		}
		valid
	}
}

impl Block {
	/*
	 * Allocates a new block of memory with the minimum available size `min_size` in bytes.
	 * The buddy allocator must be initialized before using this function.
	 */
	fn new(min_size: usize) -> Result<&'static mut Self, ()> {
		let total_min_size = core::mem::size_of::<Block>() + min_size;
		let order = buddy::get_order(util::ceil_division(total_min_size, PAGE_SIZE));
		let first_chunk_size = buddy::get_frame_size(order) - core::mem::size_of::<Block>();

		let ptr = buddy::alloc_kernel(order)?;
		let block = unsafe { &mut *(ptr as *mut Block) };
		*block = Self {
			list: LinkedList::new_single(),
			order: order,
			first_chunk: Chunk::new_free(first_chunk_size),
		};
		Ok(block)
	}

	/*
	 * Returns the total size of the block in bytes.
	 */
	fn get_total_size(&self) -> usize {
		buddy::get_frame_size(self.order)
	}
}

/*
 * List storing allocated blocks of memory.
 */
static mut BLOCKS: MaybeUninit<[Option<&'static mut Block>; 3]> = MaybeUninit::uninit();
/*
 * List storing allocated blocks of memory.
 */
static mut FREE_LISTS: MaybeUninit<[Option<&'static mut Chunk>; FREE_LIST_BINS]>
	= MaybeUninit::uninit();

/*
 * Initializes the allocator. This function must be called before using the allocator's functions
 * and exactly once.
 */
pub fn init() {
	unsafe {
		util::zero_object(&mut BLOCKS);
		util::zero_object(&mut FREE_LISTS);
	}
}

/*
 * Returns the free list for the given size `size`. If `insert` is not set, the function may return
 * a free list that contain chunks greater than the required size so that it can be split.
 */
fn get_free_list(_size: usize, _insert: bool) -> Option<&'static mut Chunk> {
	// TODO
	None
}

// TODO Mutex
/*
 * Allocates `n` bytes of kernel memory and returns a pointer to the beginning of the allocated
 * chunk. If the allocation fails, the function shall return None.
 */
pub fn alloc(_n: usize) -> Option<*mut c_void> {
	// TODO
	None
}

// TODO Mutex
/*
 * Changes the size of the memory previously allocated with `alloc`. `ptr` is the pointer to the
 * chunk of memory. `n` is the new size of the chunk of memory. If the reallocation fails, the
 * chunk is left untouched.
 */
pub fn realloc(_ptr: *const c_void, _n: usize) -> Option<*mut c_void> {
	// TODO
	None
}

// TODO Mutex
/*
 * Frees the memory at the pointer `ptr` previously allocated with `alloc`. Subsequent uses of the
 * associated memory are undefined.
 */
pub fn free(_ptr: *const c_void) {
	// TODO
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn alloc_free0() {
		if let Some(ptr) = alloc(1) {
			unsafe {
				util::memset(ptr, -1, 1);
			}
			free(ptr);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn alloc_free1() {
		if let Some(ptr) = alloc(8) {
			unsafe {
				util::memset(ptr, -1, 8);
			}
			free(ptr);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn alloc_free2() {
		if let Some(ptr) = alloc(PAGE_SIZE) {
			unsafe {
				util::memset(ptr, -1, PAGE_SIZE);
			}
			free(ptr);
		} else {
			assert!(false);
		}
	}

	// TODO
}
