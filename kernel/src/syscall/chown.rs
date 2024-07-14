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

//! The `chown` system call changes the owner of a file.

use crate::{
	file::{path::PathBuf, vfs, vfs::ResolutionSettings},
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
};

/// Performs the `chown` syscall.
pub fn do_chown(
	pathname: SyscallString,
	owner: c_int,
	group: c_int,
	rs: ResolutionSettings,
) -> EResult<usize> {
	// Validation
	if owner < -1 || group < -1 {
		return Err(errno!(EINVAL));
	}
	let path = pathname.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	// Get file
	let file_mutex = vfs::get_file_from_path(&path, &rs)?;
	let mut file = file_mutex.lock();
	// TODO allow changing group to any group whose owner is member
	if !rs.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}
	if owner > -1 {
		file.stat.set_uid(owner as _);
	}
	if group > -1 {
		file.stat.set_gid(group as _);
	}
	// TODO lazy
	file.sync()?;
	Ok(0)
}

pub fn chown(
	Args((pathname, owner, group)): Args<(SyscallString, c_int, c_int)>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	do_chown(pathname, owner, group, rs)
}
