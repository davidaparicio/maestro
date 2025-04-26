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

//! `_newselect` is similar to `select`.

use super::select::{do_select, FDSet};
use crate::{
	file::fd::FileDescriptorTable,
	process::mem_space::{copy::UserPtr, MemSpace},
	sync::mutex::Mutex,
	syscall::Args,
	time::unit::Timeval,
};
use core::ffi::c_int;
use utils::{errno::EResult, ptr::arc::Arc};

#[allow(clippy::type_complexity)]
pub fn _newselect(
	Args((nfds, readfds, writefds, exceptfds, timeout)): Args<(
		c_int,
		UserPtr<FDSet>,
		UserPtr<FDSet>,
		UserPtr<FDSet>,
		UserPtr<Timeval>,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_select(fds, nfds as _, readfds, writefds, exceptfds, timeout, None)
}
