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

//! Tmpfs (Temporary file system) is, as its name states a temporary filesystem.
//!
//! The files are stored on the kernel's memory and thus are removed when the
//! filesystem is unmounted.

use super::{kernfs, kernfs::KernFS, Filesystem, FilesystemType, NodeOps};
use crate::file::{
	fs::{kernfs::node::DefaultNode, Statfs},
	path::PathBuf,
	perm::{ROOT_GID, ROOT_UID},
	FileType, INode, Stat,
};
use core::mem::size_of;
use utils::{boxed::Box, errno, errno::EResult, io::IO, lock::Mutex, ptr::arc::Arc};

// TODO count memory usage to enforce quota

/// The default maximum amount of memory the filesystem can use in bytes.
const DEFAULT_MAX_SIZE: usize = 512 * 1024 * 1024;

/// A temporary file system.
///
/// On the inside, the tmpfs works using a kernfs.
#[derive(Debug)]
pub struct TmpFS {
	/// The maximum amount of memory in bytes the filesystem can use.
	max_size: usize,
	/// The currently used amount of memory in bytes.
	size: usize,
	/// Tells whether the filesystem is readonly.
	readonly: bool,
	/// The inner kernfs.
	inner: KernFS,
}

impl TmpFS {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `max_size` is the maximum amount of memory the filesystem can use in bytes.
	/// - `readonly` tells whether the filesystem is readonly.
	pub fn new(max_size: usize, readonly: bool) -> EResult<Self> {
		let root = DefaultNode::new(
			Stat {
				file_type: FileType::Directory,
				mode: 0o777,
				nlink: 0,
				uid: ROOT_UID,
				gid: ROOT_GID,
				size: 0,
				blocks: 0,
				dev_major: 0,
				dev_minor: 0,
				ctime: 0,
				mtime: 0,
				atime: 0,
			},
			Some(kernfs::ROOT_INODE),
			Some(kernfs::ROOT_INODE),
		)?;
		let fs = Self {
			max_size,
			// Size of the root node
			size: size_of::<DefaultNode>(),
			readonly,
			inner: KernFS::new(false, Box::new(root)?)?,
		};
		Ok(fs)
	}

	/// Executes the given function `f`.
	///
	/// On success, the function adds `s` to the total size of the filesystem.
	///
	/// If `f` fails, the function doesn't change the total size and returns the
	/// error.
	///
	/// If the new total size is too large, `f` is not executed and the
	/// function returns an error.
	fn update_size<F: FnOnce(&mut Self) -> EResult<()>>(&mut self, s: isize, f: F) -> EResult<()> {
		if s < 0 {
			f(self)?;
			self.size = self.size.saturating_sub(-s as _);
			Ok(())
		} else if self.size + (s as usize) < self.max_size {
			f(self)?;
			self.size += s as usize;
			Ok(())
		} else {
			// Quota has been reached
			Err(errno!(ENOSPC))
		}
	}
}

impl Filesystem for TmpFS {
	fn get_name(&self) -> &[u8] {
		b"tmpfs"
	}

	fn is_readonly(&self) -> bool {
		self.readonly
	}

	fn use_cache(&self) -> bool {
		self.inner.use_cache()
	}

	fn get_root_inode(&self) -> INode {
		self.inner.get_root_inode()
	}

	fn get_stat(&self) -> EResult<Statfs> {
		self.inner.get_stat()
	}

	fn load_file(&self, inode: INode) -> EResult<Box<dyn NodeOps>> {
		self.inner.load_file(inode)
	}
}

/// The tmpfs filesystem type.
pub struct TmpFsType;

impl FilesystemType for TmpFsType {
	fn get_name(&self) -> &'static [u8] {
		b"tmpfs"
	}

	fn detect(&self, _io: &mut dyn IO) -> EResult<bool> {
		Ok(false)
	}

	fn load_filesystem(
		&self,
		_io: Option<Arc<Mutex<dyn IO>>>,
		_mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Arc<dyn Filesystem>> {
		Ok(Arc::new(TmpFS::new(DEFAULT_MAX_SIZE, readonly)?)?)
	}
}
