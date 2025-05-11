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

//! A Block Group Descriptor is a structure stored in the Block Group Descriptor
//! Table which represents a block group, which is a subdivision of the
//! filesystem.

use super::{read_block, Ext2Fs};
use crate::memory::cache::RcFrameVal;
use core::{mem::size_of, sync::atomic::AtomicU16};
use macros::AnyRepr;
use utils::errno::EResult;

/// Start block of the block group descriptor table
const BGDT_START_BLK: u32 = 1;

/// A block group descriptor.
#[repr(C)]
#[derive(AnyRepr)]
pub struct BlockGroupDescriptor {
	/// The block address of the block usage bitmap.
	pub bg_block_bitmap: u32,
	/// The block address of the inode usage bitmap.
	pub bg_inode_bitmap: u32,
	/// Starting block address of inode table.
	pub bg_inode_table: u32,
	/// Number of unallocated blocks in group.
	pub bg_free_blocks_count: AtomicU16,
	/// Number of unallocated inodes in group.
	pub bg_free_inodes_count: AtomicU16,
	/// Number of directories in group.
	pub bg_used_dirs_count: AtomicU16,

	pub bg_pad: [u8; 14],
}

impl BlockGroupDescriptor {
	/// Returns the `i`th block group descriptor
	pub fn get(i: u32, fs: &Ext2Fs) -> EResult<RcFrameVal<Self>> {
		let blk_size = fs.sp.get_block_size() as usize;
		let bgd_per_blk = blk_size / size_of::<Self>();
		// Read block
		let blk_off = BGDT_START_BLK + (i / bgd_per_blk as u32);
		let blk = read_block(fs, blk_off as _)?;
		// Get entry
		let off = i as usize % bgd_per_blk;
		Ok(RcFrameVal::new(blk, off))
	}
}
