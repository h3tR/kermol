ENTRY(_start)

SECTIONS{

    . = 0xffffffff80000000;

    _limine_reqs_start = .;
    .limine_reqs : {
        KEEP(*(.limine_req_start))
        KEEP(*(.limine_reqs))
        KEEP(*(.limine_req_end))
    }

    . = ALIGN(CONSTANT(MAXPAGESIZE));

    _text_start = .;
    .text BLOCK(4K) : ALIGN(4K)
    {
        *(.text)
    }
    . = ALIGN(CONSTANT(MAXPAGESIZE));

    _rodata_start = .;
    /*Read-only data*/
    .rodata BLOCK(4K) : ALIGN(4K)
    {
    	*(.rodata)
    }
    . = ALIGN(CONSTANT(MAXPAGESIZE));


    _data_start = .;
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

    _elf_end = .;

     /DISCARD/ : {
            *(.note.gnu.build-id)
            *(.comment)
            *(.eh_frame)
        }
}