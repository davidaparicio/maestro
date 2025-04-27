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

//! The `access` system call allows to check access to a given file.

use crate::{
	file::{
		fd::FileDescriptorTable,
		vfs::{ResolutionSettings, Resolved},
	},
	memory::user::UserString,
	process::Process,
	sync::mutex::Mutex,
	syscall::{
		util::{
			at,
			at::{AT_EACCESS, AT_FDCWD},
		},
		Args,
	},
};
use core::ffi::c_int;
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

/// Checks for existence of the file.
const F_OK: i32 = 0;
/// Checks the file can be read.
const R_OK: i32 = 4;
/// Checks the file can be written.
const W_OK: i32 = 2;
/// Checks the file can be executed.
const X_OK: i32 = 1;

/// Performs the access operation.
///
/// Arguments:
/// - `dirfd` is the file descriptor of the directory relative to which the check is done.
/// - `pathname` is the path to the file.
/// - `mode` is a bitfield of access permissions to check.
/// - `flags` is a set of flags.
/// - `rs` is the process's resolution settings.
/// - `fds_mutex` is the file descriptor table.
pub fn do_access(
	dirfd: Option<i32>,
	pathname: UserString,
	mode: i32,
	flags: Option<i32>,
	rs: ResolutionSettings,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let flags = flags.unwrap_or(0);
	// Use effective IDs instead of real IDs
	let eaccess = flags & AT_EACCESS != 0;
	let ap = rs.access_profile;
	let file = {
		let fds = fds_mutex.lock();
		let pathname = pathname
			.copy_from_user()?
			.map(PathBuf::try_from)
			.transpose()?;
		let Resolved::Found(file) = at::get_file(
			&fds,
			rs,
			dirfd.unwrap_or(AT_FDCWD),
			pathname.as_deref(),
			flags,
		)?
		else {
			return Err(errno!(ENOENT));
		};
		file
	};
	// Do access checks
	let stat = file.stat();
	if (mode & R_OK != 0) && !ap.check_read_access(&stat, eaccess) {
		return Err(errno!(EACCES));
	}
	if (mode & W_OK != 0) && !ap.check_write_access(&stat, eaccess) {
		return Err(errno!(EACCES));
	}
	if (mode & X_OK != 0) && !ap.check_execute_access(&stat, eaccess) {
		return Err(errno!(EACCES));
	}
	Ok(0)
}

pub fn access(
	Args((pathname, mode)): Args<(UserString, c_int)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_access(None, pathname, mode, None, rs, fds)
}
