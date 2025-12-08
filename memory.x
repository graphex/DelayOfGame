/* Memory layout for STM32H750IBK6 on Daisy Pod */

MEMORY
{
  /* Internal Flash - 128KB */
  /* Note: Daisy typically boots from QSPI flash, but we define this for completeness */
  FLASH (rx)      : ORIGIN = 0x08000000, LENGTH = 128K

  /* DTCM RAM - 128KB (Data Tightly Coupled Memory) */
  /* Fastest memory, great for stack and critical data */
  DTCMRAM (rwx)   : ORIGIN = 0x20000000, LENGTH = 128K

  /* AXI SRAM - 512KB (total, split into regions) */
  /* Main SRAM for program data */
  RAM (rwx)       : ORIGIN = 0x24000000, LENGTH = 512K

  /* SRAM1 - 128KB */
  SRAM1 (rwx)     : ORIGIN = 0x30000000, LENGTH = 128K

  /* SRAM2 - 128KB */
  SRAM2 (rwx)     : ORIGIN = 0x30020000, LENGTH = 128K

  /* SRAM3 - 32KB */
  SRAM3 (rwx)     : ORIGIN = 0x30040000, LENGTH = 32K

  /* SRAM4 - 64KB */
  SRAM4 (rwx)     : ORIGIN = 0x38000000, LENGTH = 64K

  /* External SDRAM - 64MB (IS42S16160-based) */
  /* This is where we'll put our audio delay buffer */
  SDRAM (rwx)     : ORIGIN = 0xC0000000, LENGTH = 64M

  /* External QSPI Flash - 64MB (W25Q512JV) */
  /* Typically used for Execute-In-Place (XIP) */
  QSPI (rx)       : ORIGIN = 0x90000000, LENGTH = 64M
}

/* Default memory region for .text and .rodata */
REGION_ALIAS("REGION_TEXT", RAM);
REGION_ALIAS("REGION_RODATA", RAM);

/* Default memory region for .data and .bss */
REGION_ALIAS("REGION_DATA", RAM);
REGION_ALIAS("REGION_BSS", RAM);

/* Heap grows upward, stack grows downward */
REGION_ALIAS("REGION_HEAP", RAM);
REGION_ALIAS("REGION_STACK", DTCMRAM);

/* Define the start of the stack (top of DTCM RAM) */
_stack_start = ORIGIN(DTCMRAM) + LENGTH(DTCMRAM);
