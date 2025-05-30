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

/*
 * The linker script for the x86 architecture.
 *
 * The kernel image is split into two parts:
 * - The boot part, in lower memory (sections with the `.boot` prefix)
 * - The kernel part, in higher memory, starting at 0xc0000000
 *
 * In the first boot stage of the kernel, the memory must be remapped so that
 * the kernel image is relocated to higher memory.
 * After running the kernel code, the booting code isn't useful anymore.
 *
 * Sections need to be aligned on the page boundary to be protected against
 * writing (for those where it applies).
 *
 * BSS sections are located right after read-only sections to limit damages if
 * the stack(s) they contain overflows.
 */

ENTRY(multiboot_entry)

SECTIONS
{
    /* Boot stub sections */
	. = 0x100000;

	.boot.text : ALIGN(4K)
	{
		*(.boot.text)
	}

	.boot.data : ALIGN(4K)
	{
		*(.boot.data)
	}

	.boot.stack : ALIGN(4K)
	{
		*(.boot.stack)
	}

    /* Kernel sections */
	. = 0xc0200000;

	.text : AT (ADDR (.text) - 0xc0000000) ALIGN(4K)
	{
		*(.text*)
	}

	.rodata : AT (ADDR (.rodata) - 0xc0000000) ALIGN(4K)
	{
		*(.rodata*)
	}

    /* Accessible to the userspace (readonly) */
	.user : AT (ADDR (.user) - 0xc0000000) ALIGN(4K)
	{
	    *(.user*)
	}

	.bss : AT (ADDR (.bss) - 0xc0000000) ALIGN(4K)
	{
		*(COMMON)
		*(.bss*)
	}

	.data : AT (ADDR (.data) - 0xc0000000) ALIGN(4K)
	{
		*(.data*)
	}
}
