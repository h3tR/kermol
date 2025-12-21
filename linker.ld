ENTRY(_start)

SECTIONS{

    . = 0xffffffff80000000;

    .limine_reqs : {
        KEEP(*(.limine_req_start))
        KEEP(*(.limine_reqs))
        KEEP(*(.limine_req_end))
    }

    . = ALIGN(CONSTANT(MAXPAGESIZE));

    .text BLOCK(4K) : ALIGN(4K)
    {
        *(.text)
    }

    .rodata BLOCK(4K) : ALIGN(4K)
    {
    	*(.rodata)
    }

    	/* Read-write data (initialized) */
    .data BLOCK(4K) : ALIGN(4K)
    {
    	*(.data)
    }

    	/* Read-write data (uninitialized) and stack */
    .bss BLOCK(4K) : ALIGN(4K)
    {
    	*(COMMON)
    	*(.bss)
    }

     /DISCARD/ : {
            *(.note.gnu.build-id)
            *(.comment)
            *(.eh_frame)
        }
}