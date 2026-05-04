ENTRY(_start)

SECTIONS{

    . = 0xffffffff80000000;

    _limine_reqs_start = .;
    .limine_reqs : {
        KEEP(*(.limine_req_start))
        KEEP(*(.limine_reqs))
        KEEP(*(.limine_req_end))
    }

    _text_start = ALIGN(CONSTANT(MAXPAGESIZE));

    .text BLOCK(4K) : ALIGN(4K)
    {
        *(.text)
        *(.text.*)
    }
    _rodata_start = ALIGN(CONSTANT(MAXPAGESIZE));

    /*Read-only data*/
    .rodata BLOCK(4K) : ALIGN(4K)
    {
    	*(.rodata)
    	*(.rodata.*)
    }
    _data_start = ALIGN(CONSTANT(MAXPAGESIZE));

    /* Read-write data (initialized) */
    .data BLOCK(4K) : ALIGN(4K)
    {
    	*(.data)
    	*(.data.*)
    }

    /* Read-write data (uninitialized) and stack */
    .bss BLOCK(4K) : ALIGN(4K)
    {
    	*(COMMON)
    	*(.bss)
    	*(.bss.*)
    }
    _elf_end = ALIGN(CONSTANT(MAXPAGESIZE));

     /DISCARD/ : {
            *(.note.gnu.build-id)
            *(.comment)
            *(.eh_frame)
        }
}