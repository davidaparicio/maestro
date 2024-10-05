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

//! The IDT (Interrupt Descriptor Table) is a table under the x86 architecture
//! storing the list of interrupt handlers, allowing to catch and handle
//! interruptions.

pub mod pic;

use crate::syscall::syscall32;
use core::{
	arch::{asm, global_asm},
	ffi::c_void,
	mem::size_of,
	ptr::addr_of,
};
use utils::{
	interrupt,
	interrupt::{cli, sti},
};

/// Makes the interrupt switch to ring 0.
const ID_PRIVILEGE_RING_0: u8 = 0b00000000;
/// Makes the interrupt switch to ring 1.
const ID_PRIVILEGE_RING_1: u8 = 0b00000010;
/// Makes the interrupt switch to ring 2.
const ID_PRIVILEGE_RING_2: u8 = 0b00000100;
/// Makes the interrupt switch to ring 3.
const ID_PRIVILEGE_RING_3: u8 = 0b00000110;
/// Flag telling that the interrupt is present.
const ID_PRESENT: u8 = 0b00000001;

/// The IDT vector index for system calls.
pub const SYSCALL_ENTRY: usize = 0x80;
/// The number of entries into the IDT.
pub const ENTRIES_COUNT: usize = 0x81;

/// An IDT header.
#[repr(C, packed)]
struct InterruptDescriptorTable {
	/// The size of the IDT in bytes, minus 1.
	size: u16,
	/// The address to the beginning of the IDT.
	#[cfg(target_arch = "x86")]
	offset: u32,
	/// The address to the beginning of the IDT.
	#[cfg(target_arch = "x86_64")]
	offset: u64,
}

/// An IDT entry.
#[repr(C)]
#[derive(Clone, Copy)]
struct InterruptDescriptor {
	/// Bits 0..16 of the address to the handler for the interrupt.
	offset0: u16,
	/// The code segment selector to execute the interrupt.
	selector: u16,
	/// Must be set to zero.
	zero0: u8,
	/// Interrupt handler flags.
	flags: u8,
	/// Bits 16..32 of the address to the handler for the interrupt.
	offset1: u16,
	/// Bits 32..64 of the address to the handler for the interrupt.
	#[cfg(target_arch = "x86_64")]
	offset2: u32,
	/// Must be set to zero.
	#[cfg(target_arch = "x86_64")]
	zero1: u32,
}

impl InterruptDescriptor {
	/// Returns a placeholder entry.
	///
	/// This function is necessary because the `const_trait_impl` feature is currently unstable,
	/// preventing to use `Default`.
	const fn placeholder() -> Self {
		Self {
			offset0: 0,
			selector: 0,
			zero0: 0,
			flags: 0,
			offset1: 0,
			#[cfg(target_arch = "x86_64")]
			offset2: 0,
			#[cfg(target_arch = "x86_64")]
			zero1: 0,
		}
	}

	/// Creates an IDT entry.
	///
	/// Arguments:
	/// - `address` is the address of the handler.
	/// - `selector` is the segment selector to be used to handle the interrupt.
	/// - `flags` is the set of flags for the entry (see Intel documentation).
	fn new(address: *const c_void, selector: u16, flags: u8) -> Self {
		Self {
			offset0: (address as usize & 0xffff) as u16,
			selector,
			zero0: 0,
			flags,
			offset1: ((address as usize >> 16) & 0xffff) as u16,
			#[cfg(target_arch = "x86_64")]
			offset2: ((address as usize >> 32) & 0xffffffff) as u32,
			#[cfg(target_arch = "x86_64")]
			zero1: 0,
		}
	}
}

// include registers save/restore macros
global_asm!(r#".include "src/process/regs/regs.s""#);

/// Declare an error handler.
///
/// An error can be accompanied by a code, in which case the handler must be declared with the
/// `code` keyword.
macro_rules! error {
	($name:ident, $id:expr) => {
		extern "C" {
			fn $name();
		}

		#[cfg(target_arch = "x86")]
		global_asm!(
			r#"
.global {name}
.type {name}, @function

{name}:
	push ebp
	mov ebp, esp

	# Allocate space for registers and retrieve them
GET_REGS

	# Get the ring
	mov eax, [ebp + 8]
	and eax, 0b11

	# Push arguments to call event_handler
	push esp # regs
	push eax # ring
	push 0 # code
	push {id}
	call event_handler
	add esp, 16

RESTORE_REGS

	# Restore the context
	mov esp, ebp
	pop ebp
	iretd"#,
			name = sym $name,
			id = const($id)
		);
	};
	($name:ident, $id:expr, code) => {
		extern "C" {
			fn $name();
		}

		#[cfg(target_arch = "x86")]
		global_asm!(
			r#"
.global {name}
.type {name}, @function

{name}:
	# Retrieve the error code and write it after the stack pointer so that it can be retrieved
	# after the stack frame
	push eax
	mov eax, [esp + 4]
	mov [esp - 4], eax
	pop eax

	# Remove the code from its previous location on the stack
	add esp, 4

	push ebp
	mov ebp, esp

	# Allocate space for the error code
	push [esp - 8]

	# Allocate space for registers and retrieve them
GET_REGS

	# Get the ring
	mov eax, [ebp + 8]
	and eax, 0b11

	# Push arguments to call event_handler
	push esp # regs
	push eax # ring
	push [esp + REGS_SIZE + 8] # code
	push {id}
	call event_handler
	add esp, 16

RESTORE_REGS

	# Free the space allocated for the error code
	add esp, 4

	mov esp, ebp
	pop ebp
	iretd"#,
			name = sym $name,
			id = const($id)
		);
	};
}

macro_rules! irq {
	($name:ident, $id:expr) => {
		extern "C" {
			fn $name();
		}

		#[cfg(target_arch = "x86")]
		global_asm!(
			r#"
.global {name}

{name}:
	push ebp
	mov ebp, esp

	# Allocate space for registers and retrieve them
GET_REGS

	# Get the ring
	mov eax, [ebp + 8]
	and eax, 0b11

	# Push arguments to call event_handler
	push esp # regs
	push eax # ring
	push 0 # code
	push ({id} + 0x20)
	call event_handler
	add esp, 16

RESTORE_REGS

	# Restore the context
	mov ebp, ebp
	pop ebp
	iretd"#,
			name = sym $name,
			id = const($id)
		);
	};
}

error!(error0, 0);
error!(error1, 1);
error!(error2, 2);
error!(error3, 3);
error!(error4, 4);
error!(error5, 5);
error!(error6, 6);
error!(error7, 7);
error!(error8, 8, code);
error!(error9, 9);
error!(error10, 10, code);
error!(error11, 11, code);
error!(error12, 12, code);
error!(error13, 13, code);
error!(error14, 14, code);
error!(error15, 15);
error!(error16, 16);
error!(error17, 17, code);
error!(error18, 18);
error!(error19, 19);
error!(error20, 20);
error!(error21, 21);
error!(error22, 22);
error!(error23, 23);
error!(error24, 24);
error!(error25, 25);
error!(error26, 26);
error!(error27, 27);
error!(error28, 28);
error!(error29, 29);
error!(error30, 30, code);
error!(error31, 31);

irq!(irq0, 0);
irq!(irq1, 1);
irq!(irq2, 2);
irq!(irq3, 3);
irq!(irq4, 4);
irq!(irq5, 5);
irq!(irq6, 6);
irq!(irq7, 7);
irq!(irq8, 8);
irq!(irq9, 9);
irq!(irq10, 10);
irq!(irq11, 11);
irq!(irq12, 12);
irq!(irq13, 13);
irq!(irq14, 14);
irq!(irq15, 15);

/// The list of IDT entries.
static mut IDT_ENTRIES: [InterruptDescriptor; ENTRIES_COUNT] =
	[InterruptDescriptor::placeholder(); ENTRIES_COUNT];

/// Executes the given function `f` with maskable interruptions disabled.
///
/// This function saves the state of the interrupt flag and restores it before
/// returning.
pub fn wrap_disable_interrupts<T, F: FnOnce() -> T>(f: F) -> T {
	let int = interrupt::is_enabled();
	// Here is assumed that no interruption will change flags register. Which could cause a
	// race condition
	cli();
	let res = f();
	if int {
		sti();
	} else {
		cli();
	}
	res
}

/// Initializes the IDT.
///
/// This function must be called only once at kernel initialization.
///
/// When returning, maskable interrupts are disabled by default.
pub(crate) fn init() {
	cli();
	pic::init(0x20, 0x28);
	// Safe because the current function is called only once at boot
	unsafe {
		// Errors
		IDT_ENTRIES[0x00] = InterruptDescriptor::new(error0 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x01] = InterruptDescriptor::new(error1 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x02] = InterruptDescriptor::new(error2 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x03] = InterruptDescriptor::new(error3 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x04] = InterruptDescriptor::new(error4 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x05] = InterruptDescriptor::new(error5 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x06] = InterruptDescriptor::new(error6 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x07] = InterruptDescriptor::new(error7 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x08] = InterruptDescriptor::new(error8 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x09] = InterruptDescriptor::new(error9 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0a] = InterruptDescriptor::new(error10 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0b] = InterruptDescriptor::new(error11 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0c] = InterruptDescriptor::new(error12 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0d] = InterruptDescriptor::new(error13 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0e] = InterruptDescriptor::new(error14 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x0f] = InterruptDescriptor::new(error15 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x10] = InterruptDescriptor::new(error16 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x11] = InterruptDescriptor::new(error17 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x12] = InterruptDescriptor::new(error18 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x13] = InterruptDescriptor::new(error19 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x14] = InterruptDescriptor::new(error20 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x15] = InterruptDescriptor::new(error21 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x16] = InterruptDescriptor::new(error22 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x17] = InterruptDescriptor::new(error23 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x18] = InterruptDescriptor::new(error24 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x19] = InterruptDescriptor::new(error25 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1a] = InterruptDescriptor::new(error26 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1b] = InterruptDescriptor::new(error27 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1c] = InterruptDescriptor::new(error28 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1d] = InterruptDescriptor::new(error29 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1e] = InterruptDescriptor::new(error30 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x1f] = InterruptDescriptor::new(error31 as _, 0x8, 0x8e);
		// IRQ
		IDT_ENTRIES[0x20] = InterruptDescriptor::new(irq0 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x21] = InterruptDescriptor::new(irq1 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x22] = InterruptDescriptor::new(irq2 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x23] = InterruptDescriptor::new(irq3 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x24] = InterruptDescriptor::new(irq4 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x25] = InterruptDescriptor::new(irq5 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x26] = InterruptDescriptor::new(irq6 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x27] = InterruptDescriptor::new(irq7 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x28] = InterruptDescriptor::new(irq8 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x29] = InterruptDescriptor::new(irq9 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2a] = InterruptDescriptor::new(irq10 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2b] = InterruptDescriptor::new(irq11 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2c] = InterruptDescriptor::new(irq12 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2d] = InterruptDescriptor::new(irq13 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2e] = InterruptDescriptor::new(irq14 as _, 0x8, 0x8e);
		IDT_ENTRIES[0x2f] = InterruptDescriptor::new(irq15 as _, 0x8, 0x8e);
		// System calls
		IDT_ENTRIES[SYSCALL_ENTRY] = InterruptDescriptor::new(syscall32 as _, 0x8, 0xee);
		// Load
		let idt = InterruptDescriptorTable {
			size: (size_of::<InterruptDescriptor>() * ENTRIES_COUNT - 1) as u16,
			offset: addr_of!(IDT_ENTRIES) as _,
		};
		asm!("lidt [{idt}]", idt = in(reg) &idt);
	}
}
