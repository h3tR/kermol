ENTRY(_start)

SECTIONS{

    .multiboot2_header : ALIGN(8) {
            KEEP(*(.multiboot2_header))
    }

    . = 1M;

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

    .stack : ALIGN(4K) {
        . += 16K;
        STACK_TOP = .;
    }

     /DISCARD/ : {
            *(.note.gnu.build-id)
            *(.comment)
            *(.eh_frame)
        }
}