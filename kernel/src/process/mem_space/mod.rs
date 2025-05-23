/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! A memory space is a virtual memory handler for a process. It handles virtual and physical
//! memory allocations for the process, as well as linkage between them.
//!
//! The memory space contains two types of structures:
//! - Mapping: A chunk of virtual memory that is allocated
//! - Gap: A chunk of virtual memory that is available to be allocated

pub mod copy;
mod gap;
mod mapping;
mod transaction;

use crate::{
	arch::x86::paging::{PAGE_FAULT_INSTRUCTION, PAGE_FAULT_WRITE},
	file::{perm::AccessProfile, vfs, File},
	memory,
	memory::{cache::RcFrame, vmem::VMem, VirtAddr, PROCESS_END},
};
use core::{
	alloc::AllocError, cmp::min, ffi::c_void, fmt, intrinsics::unlikely, mem, num::NonZeroUsize,
};
use gap::MemGap;
use mapping::MemMapping;
use transaction::MemSpaceTransaction;
use utils::{
	collections::{btreemap::BTreeMap, vec::Vec},
	errno,
	errno::{AllocResult, CollectResult, EResult},
	limits::PAGE_SIZE,
	ptr::arc::Arc,
	range_cmp, TryClone,
};

/// Page can be read
pub const PROT_READ: u8 = 0x1;
/// Page can be written
pub const PROT_WRITE: u8 = 0x2;
/// Page can be executed
pub const PROT_EXEC: u8 = 0x4;

/// Changes are shared across mappings on the same region
pub const MAP_SHARED: u8 = 0x1;
/// Changes are *not* shared across mappings on the same region
pub const MAP_PRIVATE: u8 = 0x2;
/// Interpret `addr` exactly
pub const MAP_FIXED: u8 = 0x10;
/// The mapping is not backed by any file
pub const MAP_ANONYMOUS: u8 = 0x20;

/// The virtual address of the buffer used to map pages for copy.
const COPY_BUFFER: VirtAddr = VirtAddr(PROCESS_END.0 - PAGE_SIZE);

/// Type representing a memory page.
pub type Page = [u8; PAGE_SIZE];

/// Tells whether the address is in bound of the userspace.
pub fn bound_check(addr: usize, n: usize) -> bool {
	addr >= PAGE_SIZE && addr.saturating_add(n) <= COPY_BUFFER.0
}

// TODO Add a variant for ASLR
/// Enumeration of constraints for the selection of the virtual address for a memory mapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MapConstraint {
	/// The mapping is done at a fixed address.
	///
	/// Previous allocation(s) in the range of the allocation are unmapped.
	///
	/// The allocation is allowed to take place outside ranges that are normally allowed, but not
	/// in kernelspace.
	Fixed(VirtAddr),

	/// Providing a hint for the address to use. The allocator will try to use the address if
	/// available.
	///
	/// If not available, the constraint is ignored and another address is selected.
	Hint(VirtAddr),

	/// No constraint.
	None,
}

impl MapConstraint {
	/// Tells whether the constraint is valid.
	pub fn is_valid(self) -> bool {
		match self {
			// Checking the address is within userspace is required because `Fixed` allocations can
			// take place *outside of gaps* but *not inside the kernelspace*
			MapConstraint::Fixed(addr) => {
				// The copy buffer is located right before the kernelspace
				addr < COPY_BUFFER && addr.is_aligned_to(PAGE_SIZE)
			}
			MapConstraint::Hint(addr) => addr.is_aligned_to(PAGE_SIZE),
			_ => true,
		}
	}
}

/// Removes gaps in `on` in the given range, using `transaction`.
///
/// `start` is the start address of the range and `size` is the size of the range in pages.
fn remove_gaps_in_range(
	transaction: &mut MemSpaceTransaction,
	start: VirtAddr,
	size: usize,
) -> AllocResult<()> {
	// Start the search at the gap containing the start address
	let search_start = transaction
		.mem_space_state
		.get_gap_for_addr(start)
		.map(MemGap::get_begin)
		// No gap contain the start address, start at the next one
		.unwrap_or(start);
	// Bound the search to the end of the range
	let end = start + size * PAGE_SIZE;
	// Collect gaps that match
	let gaps = transaction
		.mem_space_state
		.gaps
		.range(search_start..end)
		.map(|(_, b)| b.clone())
		.collect::<CollectResult<Vec<_>>>()
		.0?;
	// Iterate on gaps and apply modifications
	for gap in gaps {
		let gap_begin = gap.get_begin();
		let gap_end = gap.get_end();
		// Compute range to remove
		let start = start.0.clamp(gap_begin.0, gap_end.0);
		let end = end.0.clamp(gap_begin.0, gap_end.0);
		// Rounding is not a problem because all values are multiples of the page size
		let size = (end - start) / PAGE_SIZE;
		// Consume the gap and store new gaps
		let (prev, next) = gap.consume(start, size);
		transaction.remove_gap(gap_begin)?;
		if let Some(g) = prev {
			transaction.insert_gap(g)?;
		}
		if let Some(g) = next {
			transaction.insert_gap(g)?;
		}
	}
	Ok(())
}

/// Inner state of the memory space, to use as a model for the virtual memory context.
#[derive(Debug, Default)]
struct MemSpaceState {
	/// Binary tree storing the list of memory gaps, ready for new mappings.
	///
	/// The collection is sorted by pointer to the beginning of the mapping on the virtual
	/// memory.
	gaps: BTreeMap<VirtAddr, MemGap>,
	/// Binary tree storing the list of memory mappings.
	///
	/// Sorted by pointer to the beginning of the mapping on the virtual memory.
	mappings: BTreeMap<*mut u8, MemMapping>,

	/// The number of used virtual memory pages.
	vmem_usage: usize,
}

impl MemSpaceState {
	/// Returns a reference to a gap with at least size `size`.
	///
	/// `size` is the minimum size of the gap to be returned.
	///
	/// If no gap large enough is available, the function returns `None`.
	fn get_gap(&self, size: NonZeroUsize) -> Option<&MemGap> {
		self.gaps
			.iter()
			.map(|(_, g)| g)
			.find(|g| g.get_size() >= size)
	}

	/// Returns a reference to the gap containing the given virtual address.
	///
	/// If no gap contain the pointer, the function returns `None`.
	fn get_gap_for_addr(&self, addr: VirtAddr) -> Option<&MemGap> {
		self.gaps
			.cmp_get(|key, value| range_cmp(key.0, value.get_size().get() * PAGE_SIZE, addr.0))
	}

	/// Returns an immutable reference to the memory mapping containing the given virtual
	/// address.
	///
	/// If no mapping contains the address, the function returns `None`.
	pub fn get_mapping_for_addr(&self, addr: VirtAddr) -> Option<&MemMapping> {
		self.mappings.cmp_get(|key, value| {
			range_cmp(*key as usize, value.get_size().get() * PAGE_SIZE, addr.0)
		})
	}

	/// Returns a mutable reference to the memory mapping containing the given virtual
	/// address.
	///
	/// If no mapping contains the address, the function returns `None`.
	pub fn get_mut_mapping_for_addr(&mut self, addr: VirtAddr) -> Option<&mut MemMapping> {
		self.mappings.cmp_get_mut(|key, value| {
			range_cmp(*key as usize, value.get_size().get() * PAGE_SIZE, addr.0)
		})
	}
}

/// Executable program information.
#[derive(Clone)]
pub struct ExeInfo {
	/// The VFS entry of the program loaded on this memory space.
	pub exe: Arc<vfs::Entry>,

	/// Address to the beginning of program argument.
	pub argv_begin: VirtAddr,
	/// Address to the end of program argument.
	pub argv_end: VirtAddr,
	/// Address to the beginning of program environment.
	pub envp_begin: VirtAddr,
	/// Address to the end of program environment.
	pub envp_end: VirtAddr,
}

/// A virtual memory space.
pub struct MemSpace {
	/// The memory space's structure, used as a model for `vmem`.
	state: MemSpaceState,
	/// Architecture-specific virtual memory context handler.
	///
	/// We use it as a cache which can be invalidated by unmapping. When a page fault occurs, we
	/// can then fetch the actual mapping from here.
	pub vmem: VMem,

	/// The initial pointer of the `[s]brk` system calls.
	brk_init: VirtAddr,
	/// The current pointer of the `[s]brk` system calls.
	brk: VirtAddr,

	/// Executable program information.
	pub exe_info: ExeInfo,
}

impl MemSpace {
	/// Creates a new virtual memory object.
	///
	/// `exe` is the VFS entry of the program loaded on the memory space.
	pub fn new(exe: Arc<vfs::Entry>) -> AllocResult<Self> {
		let mut s = Self {
			state: MemSpaceState::default(),
			vmem: unsafe { VMem::new() },

			brk_init: Default::default(),
			brk: Default::default(),

			exe_info: ExeInfo {
				exe,

				argv_begin: Default::default(),
				argv_end: Default::default(),
				envp_begin: Default::default(),
				envp_end: Default::default(),
			},
		};
		// Create the default gap of memory which is present at the beginning
		let begin = memory::ALLOC_BEGIN;
		let size = (COPY_BUFFER.0 - begin.0) / PAGE_SIZE;
		let gap = MemGap::new(begin, NonZeroUsize::new(size).unwrap());
		let mut transaction = MemSpaceTransaction::new(&mut s.state, &mut s.vmem);
		transaction.insert_gap(gap)?;
		transaction.commit();
		Ok(s)
	}

	/// Returns the number of virtual memory pages in the memory space.
	#[inline]
	pub fn get_vmem_usage(&self) -> usize {
		self.state.vmem_usage
	}

	/// Returns an immutable reference to the memory mapping containing the given virtual
	/// address.
	///
	/// If no mapping contains the address, the function returns `None`.
	#[inline]
	pub fn get_mapping_for_addr(&self, addr: VirtAddr) -> Option<&MemMapping> {
		self.state.get_mapping_for_addr(addr)
	}

	fn map_impl(
		transaction: &mut MemSpaceTransaction,
		map_constraint: MapConstraint,
		size: NonZeroUsize,
		prot: u8,
		flags: u8,
		file: Option<Arc<File>>,
		off: u64,
	) -> EResult<MemMapping> {
		if !map_constraint.is_valid() {
			return Err(errno!(ENOMEM));
		}
		// Get suitable gap for the given constraint
		let (gap, gap_off) = match map_constraint {
			MapConstraint::Fixed(addr) => {
				Self::unmap_impl(transaction, addr, size, true)?;
				// Remove gaps that are present where the mapping is to be placed
				remove_gaps_in_range(transaction, addr, size.get())?;
				// Create a fictive gap. This is required because fixed allocations may be used
				// outside allowed gaps
				let gap = MemGap::new(addr, size);
				(gap, 0)
			}
			MapConstraint::Hint(addr) => {
				transaction
					.mem_space_state
					// Get the gap for the pointer
					.get_gap_for_addr(addr)
					.and_then(|gap| {
						// Offset in the gap
						let off = gap.get_page_offset_for(addr);
						// Check whether the mapping fits in the gap
						let end = off.checked_add(size.get())?;
						(end <= gap.get_size().get()).then_some((gap.clone(), off))
					})
					// Hint cannot be satisfied. Get a large enough gap
					.or_else(|| {
						let gap = transaction.mem_space_state.get_gap(size)?;
						Some((gap.clone(), 0))
					})
					.ok_or(AllocError)?
					.clone()
			}
			MapConstraint::None => {
				let gap = transaction
					.mem_space_state
					.get_gap(size)
					.ok_or(AllocError)?
					.clone();
				(gap, 0)
			}
		};
		let addr = (gap.get_begin() + gap_off * PAGE_SIZE).as_ptr();
		// Split the old gap to fit the mapping, and insert new gaps
		let (left_gap, right_gap) = gap.consume(gap_off, size.get());
		transaction.remove_gap(gap.get_begin())?;
		if let Some(new_gap) = left_gap {
			transaction.insert_gap(new_gap)?;
		}
		if let Some(new_gap) = right_gap {
			transaction.insert_gap(new_gap)?;
		}
		// Create the mapping
		Ok(MemMapping::new(addr, size, prot, flags, file, off)?)
	}

	/// Maps a chunk of memory.
	///
	/// The function has complexity `O(log n)`.
	///
	/// Arguments:
	/// - `map_constraint` is the constraint to fulfill for the allocation
	/// - `size` is the size of the mapping in number of memory pages
	/// - `prot` is the memory protection
	/// - `flags` is the flags for the mapping
	/// - `file` is the open file the mapping points to. If `None`, no file is mapped
	/// - `off` is the offset in `file`, if applicable
	///
	/// The underlying physical memory is not allocated directly but only when an attempt to write
	/// the memory is detected.
	///
	/// On success, the function returns a pointer to the newly mapped virtual memory.
	///
	/// If the given pointer is not page-aligned, the function returns an error.
	pub fn map(
		&mut self,
		map_constraint: MapConstraint,
		size: NonZeroUsize,
		prot: u8,
		flags: u8,
		file: Option<Arc<File>>,
		off: u64,
	) -> EResult<*mut u8> {
		let mut transaction = MemSpaceTransaction::new(&mut self.state, &mut self.vmem);
		let map = Self::map_impl(
			&mut transaction,
			map_constraint,
			size,
			prot,
			flags,
			file,
			off,
		)?;
		let addr = map.get_addr();
		transaction.insert_mapping(map)?;
		transaction.commit();
		Ok(addr)
	}

	/// Maps a chunk of memory population with the given static pages.
	pub fn map_special(&mut self, prot: u8, flags: u8, pages: &[RcFrame]) -> AllocResult<*mut u8> {
		let Some(len) = NonZeroUsize::new(pages.len()) else {
			return Err(AllocError);
		};
		let mut transaction = MemSpaceTransaction::new(&mut self.state, &mut self.vmem);
		let mut map = Self::map_impl(
			&mut transaction,
			MapConstraint::None,
			len,
			prot,
			flags,
			None,
			0,
		)
		.map_err(|_| AllocError)?;
		// Populate
		map.anon_pages
			.iter_mut()
			.zip(pages.iter().cloned())
			.for_each(|(dst, src)| *dst = Some(src));
		// Commit
		let addr = map.get_addr();
		transaction.insert_mapping(map)?;
		transaction.commit();
		Ok(addr)
	}

	/// Implementation for `unmap`.
	///
	/// If `nogap` is `true`, the function does not create any gap.
	///
	/// On success, the function returns the transaction.
	fn unmap_impl(
		transaction: &mut MemSpaceTransaction,
		addr: VirtAddr,
		size: NonZeroUsize,
		nogap: bool,
	) -> EResult<()> {
		// Remove every mapping in the chunk to unmap
		let mut i = 0;
		while i < size.get() {
			// The current page's beginning
			let page_addr = addr + i * PAGE_SIZE;
			// The mapping containing the page
			let Some(mapping) = transaction.mem_space_state.get_mapping_for_addr(page_addr) else {
				// TODO jump to next mapping directly using binary tree (currently O(n log n))
				i += 1;
				continue;
			};
			// The pointer to the beginning of the mapping
			let mapping_begin = mapping.get_addr();
			// The offset in the mapping to the beginning of pages to unmap
			let inner_off = (page_addr.0 - mapping_begin as usize) / PAGE_SIZE;
			// The number of pages to unmap in the mapping
			let pages = min(size.get() - i, mapping.get_size().get() - inner_off);
			i += pages;
			// Newly created mappings and gap after removing parts of the previous one
			let (prev, gap, next) = mapping.split(inner_off, pages)?;
			// Remove the old mapping and insert new ones
			transaction.remove_mapping(mapping_begin)?;
			if let Some(m) = prev {
				transaction.insert_mapping(m)?;
			}
			if let Some(m) = next {
				transaction.insert_mapping(m)?;
			}
			if nogap {
				continue;
			}
			// Insert gap
			if let Some(mut gap) = gap {
				// Merge previous gap
				let prev_gap = (!gap.get_begin().is_null())
					.then(|| {
						let prev_gap_ptr = gap.get_begin() - 1;
						transaction.mem_space_state.get_gap_for_addr(prev_gap_ptr)
					})
					.flatten()
					.cloned();
				if let Some(p) = prev_gap {
					transaction.remove_gap(p.get_begin())?;
					gap.merge(&p);
				}
				// Merge next gap
				let next_gap = transaction
					.mem_space_state
					.get_gap_for_addr(gap.get_end())
					.cloned();
				if let Some(n) = next_gap {
					transaction.remove_gap(n.get_begin())?;
					gap.merge(&n);
				}
				transaction.insert_gap(gap)?;
			}
		}
		Ok(())
	}

	/// Unmaps the given mapping of memory.
	///
	/// Arguments:
	/// - `addr` represents the aligned address of the beginning of the chunk to unmap.
	/// - `size` represents the size of the mapping in number of memory pages.
	/// - `brk` tells whether the function is called through the `brk` syscall.
	///
	/// The function frees the physical memory the mapping points to
	/// unless shared by one or several other memory mappings.
	///
	/// After this function returns, the access to the mapping of memory shall
	/// be revoked and further attempts to access it shall result in a page
	/// fault.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn unmap(&mut self, addr: VirtAddr, size: NonZeroUsize, brk: bool) -> EResult<()> {
		// Validation
		if unlikely(!addr.is_aligned_to(PAGE_SIZE)) {
			return Err(errno!(ENOMEM));
		}
		let mut transaction = MemSpaceTransaction::new(&mut self.state, &mut self.vmem);
		// Do not create gaps if unmapping for `*brk` system calls as this space is reserved by
		// it and must not be reused by `mmap`
		Self::unmap_impl(&mut transaction, addr, size, brk)?;
		transaction.commit();
		Ok(())
	}

	/// Binds the memory space to the current kernel.
	pub fn bind(&self) {
		self.vmem.bind();
	}

	/// Tells whether the memory space is bound.
	pub fn is_bound(&self) -> bool {
		self.vmem.is_bound()
	}

	/// Clones the current memory space for process forking.
	pub fn fork(&mut self) -> EResult<MemSpace> {
		// Clone first to mark as shared
		let mappings = self.state.mappings.try_clone()?;
		// Unmap to invalidate the virtual memory context
		for (_, m) in &self.state.mappings {
			self.vmem
				.unmap_range(VirtAddr::from(m.get_addr()), m.get_size().get());
		}
		Ok(Self {
			state: MemSpaceState {
				gaps: self.state.gaps.try_clone()?,
				mappings,

				vmem_usage: self.state.vmem_usage,
			},
			vmem: unsafe { VMem::new() },

			brk_init: self.brk_init,
			brk: self.brk,

			exe_info: self.exe_info.clone(),
		})
	}

	/// Allocates the physical pages on the given range.
	///
	/// Arguments:
	/// - `addr` is the virtual address to beginning of the range to allocate.
	/// - `len` is the size of the range in bytes.
	///
	/// If the mapping does not exist, the function returns an error.
	///
	/// On error, allocations that have been made are not freed as it does not affect the behaviour
	/// from the user's point of view.
	pub fn alloc(&mut self, addr: VirtAddr, len: usize) -> EResult<()> {
		let mut off = 0;
		while off < len {
			let addr = addr + off;
			if let Some(mapping) = self.state.get_mut_mapping_for_addr(addr) {
				let page_offset = (addr.0 - mapping.get_addr() as usize) / PAGE_SIZE;
				mapping.map(page_offset, &mut self.vmem)?;
			}
			off += PAGE_SIZE;
		}
		Ok(())
	}

	/// Sets protection for the given range of memory.
	///
	/// Arguments:
	/// - `addr` is the address to the beginning of the range to be set
	/// - `len` is the length of the range in bytes
	/// - `prot` is a set of mapping flags
	/// - `access_profile` is the access profile to check permissions
	///
	/// If a mapping to be modified is associated with a file, and the file doesn't have the
	/// matching permissions, the function returns an error.
	pub fn set_prot(
		&mut self,
		_addr: *mut c_void,
		_len: usize,
		_prot: u8,
		_access_profile: &AccessProfile,
	) -> EResult<()> {
		// TODO Iterate on mappings in the range:
		//		If the mapping is shared and associated to a file, check file permissions match
		// `prot` (only write)
		//		Split the mapping if needed
		//		Set permissions
		//		Update vmem
		Ok(())
	}

	/// Returns the address for the `brk` syscall.
	pub fn get_brk(&self) -> VirtAddr {
		self.brk
	}

	/// Sets the initial pointer for the `brk` syscall.
	///
	/// This function MUST be called *only once*, before the program starts.
	///
	/// `addr` MUST be page-aligned.
	pub fn set_brk_init(&mut self, addr: VirtAddr) {
		debug_assert!(addr.is_aligned_to(PAGE_SIZE));
		self.brk_init = addr;
		self.brk = addr;
	}

	/// Sets the address for the `brk` syscall.
	///
	/// If the memory cannot be allocated, the function returns an error.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn set_brk(&mut self, addr: VirtAddr) -> AllocResult<()> {
		if addr >= self.brk {
			// Check the pointer is valid
			if addr > COPY_BUFFER {
				return Err(AllocError);
			}
			// Allocate memory
			let begin = self.brk.align_to(PAGE_SIZE);
			let pages = (addr.0 - begin.0).div_ceil(PAGE_SIZE);
			let Some(pages) = NonZeroUsize::new(pages) else {
				return Ok(());
			};
			self.map(
				MapConstraint::Fixed(begin),
				pages,
				PROT_READ | PROT_WRITE | PROT_EXEC,
				MAP_ANONYMOUS,
				None,
				0,
			)
			.map_err(|_| AllocError)?;
		} else {
			// Check the pointer is valid
			if unlikely(addr < self.brk_init) {
				return Err(AllocError);
			}
			// Free memory
			let begin = addr.align_to(PAGE_SIZE);
			let pages = (begin.0 - addr.0).div_ceil(PAGE_SIZE);
			let Some(pages) = NonZeroUsize::new(pages) else {
				return Ok(());
			};
			self.unmap(begin, pages, true).map_err(|_| AllocError)?;
		}
		self.brk = addr;
		Ok(())
	}

	/// Function called whenever the CPU triggered a page fault for the context.
	///
	/// This function determines whether the process should continue or not.
	///
	/// If continuing, the function must resolve the issue before returning.
	/// A typical situation where is function is useful is for Copy-On-Write allocations.
	///
	/// Arguments:
	/// - `addr` is the virtual address of the wrong memory access that caused the fault.
	/// - `code` is the error code given along with the error.
	///
	/// If the process should continue, the function returns `true`, else `false`.
	pub fn handle_page_fault(&mut self, addr: VirtAddr, code: u32) -> EResult<bool> {
		let Some(mapping) = self.state.get_mut_mapping_for_addr(addr) else {
			return Ok(false);
		};
		// Check permissions
		let prot = mapping.get_prot();
		if unlikely(code & PAGE_FAULT_WRITE != 0 && prot & PROT_WRITE == 0) {
			return Ok(false);
		}
		if unlikely(code & PAGE_FAULT_INSTRUCTION != 0 && prot & PROT_EXEC == 0) {
			return Ok(false);
		}
		// Map the accessed page
		let page_offset = (addr.0 - mapping.get_addr() as usize) / PAGE_SIZE;
		mapping.map(page_offset, &mut self.vmem)?;
		Ok(true)
	}
}

impl fmt::Debug for MemSpace {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&self.state, f)
	}
}

impl Drop for MemSpace {
	fn drop(&mut self) {
		// Synchronize all mappings to disk
		let mappings = mem::take(&mut self.state.mappings);
		for (_, m) in mappings {
			// Ignore I/O errors
			let _ = m.sync(&self.vmem, true);
		}
	}
}
