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

//! The `clock_gettime` syscall returns the current time of the given clock.

use crate::{
	process::{mem_space::copy::SyscallPtr, Process},
	syscall::Args,
	time::{
		clock,
		clock::Clock,
		unit::{ClockIdT, TimeUnit, Timespec},
	},
};
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn clock_gettime(
	Args((clockid, tp)): Args<(ClockIdT, SyscallPtr<Timespec>)>,
) -> EResult<usize> {
	let clk = Clock::from_id(clockid).ok_or_else(|| errno!(EINVAL))?;
	let ts = clock::current_time_ns(clk);
	tp.copy_to_user(&Timespec::from_nano(ts))?;
	Ok(0)
}
