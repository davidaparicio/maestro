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

//! The `timer_create` system call creates a per-process timer.

use crate::{
	process::{
		mem_space::copy::SyscallPtr,
		signal::{SigEvent, Signal, SIGEV_SIGNAL},
		Process,
	},
	syscall::Args,
	time::{
		clock::Clock,
		unit::{ClockIdT, TimerT},
	},
};
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn timer_create(
	Args((clockid, sevp, timerid)): Args<(ClockIdT, SyscallPtr<SigEvent>, SyscallPtr<TimerT>)>,
	proc: Arc<Process>,
) -> EResult<usize> {
	let clock = Clock::from_id(clockid).ok_or_else(|| errno!(EINVAL))?;
	let timerid_val = timerid.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let sevp_val = sevp.copy_from_user()?.unwrap_or_else(|| SigEvent {
		sigev_notify: SIGEV_SIGNAL,
		sigev_signo: Signal::SIGALRM as _,
		sigev_value: timerid_val,
		sigev_notify_function: None,
		sigev_notify_attributes: None,
		sigev_notify_thread_id: proc.tid,
	});
	let id = proc.timer_manager.lock().create_timer(clock, sevp_val)?;
	timerid.copy_to_user(&(id as _))?;
	Ok(0)
}
