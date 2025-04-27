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

//! The `read` system call allows to read the content of an open file.

use super::Args;
use crate::{
	file::{fd::FileDescriptorTable, FileType},
	memory::user::UserSlice,
	process::{scheduler, Process},
	sync::mutex::Mutex,
};
use core::{cmp::min, ffi::c_int, sync::atomic};
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
	vec,
};

pub fn read(
	Args((fd, buf, count)): Args<(c_int, *mut u8, usize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, count)?;
	// Validation
	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}
	let file = fds.lock().get_fd(fd)?.get_file().clone();
	if file.get_type()? == FileType::Link {
		return Err(errno!(EINVAL));
	}
	// Read
	let off = file.off.load(atomic::Ordering::Acquire);
	let len = file.ops.read(&file, off, buf)?;
	// Update offset
	let new_off = off.saturating_add(len as u64);
	file.off.store(new_off, atomic::Ordering::Release);
	Ok(len as _)
}
