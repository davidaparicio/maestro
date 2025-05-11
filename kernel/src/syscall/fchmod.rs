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

//! The `fchmod` system call allows change the permissions on a file.

use crate::{
	file,
	file::{fd::FileDescriptorTable, fs::StatSet, perm::AccessProfile, vfs},
	process::Process,
	sync::mutex::Mutex,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn fchmod(
	Args((fd, mode)): Args<(c_int, file::Mode)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	ap: AccessProfile,
) -> EResult<usize> {
	let file = fds_mutex
		.lock()
		.get_fd(fd)?
		.get_file()
		.vfs_entry
		.clone()
		.ok_or_else(|| errno!(EROFS))?;
	// Check permissions
	let stat = file.stat();
	if !ap.can_set_file_permissions(&stat) {
		return Err(errno!(EPERM));
	}
	vfs::set_stat(
		file.node(),
		&StatSet {
			mode: Some(mode),
			..Default::default()
		},
	)?;
	Ok(0)
}
