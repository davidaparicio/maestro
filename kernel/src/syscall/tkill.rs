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

//! The `tkill` system call allows to send a signal to a specific thread.

use crate::{
	file::perm::AccessProfile,
	process::{Process, pid::Pid, signal::Signal},
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn tkill(
	Args((tid, sig)): Args<(Pid, c_int)>,
	access_profile: AccessProfile,
) -> EResult<usize> {
	let signal = Signal::try_from(sig)?;
	let thread = Process::get_by_tid(tid).ok_or(errno!(ESRCH))?;
	if !access_profile.can_kill(&thread) {
		return Err(errno!(EPERM));
	}
	thread.kill(signal);
	Ok(0)
}
