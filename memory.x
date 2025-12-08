/* Memory layout for STM32H750IBK6 on Daisy Pod/Seed */
/* Based on daisy-embassy memory configuration */

MEMORY
{
  /* Internal Flash - 128KB (often unused, boots from QSPI) */
  FLASH (rx)      : ORIGIN = 0x08000000, LENGTH = 128K

  /* ITCM RAM - 64KB (Instruction Tightly Coupled Memory) */
  ITCMRAM (rwx)   : ORIGIN = 0x00000000, LENGTH = 64K

  /* DTCM RAM - 128KB (Data Tightly Coupled Memory) */
  /* Fastest memory for stack and time-critical data */
  DTCMRAM (rwx)   : ORIGIN = 0x20000000, LENGTH = 128K

  /* AXI SRAM - 512KB */
  /* Main program memory */
  SRAM (rwx)      : ORIGIN = 0x24000000, LENGTH = 512K

  /* AHB SRAM (D2 domain) - 288KB total */
  /* SRAM1: 128KB, SRAM2: 128KB, SRAM3: 32KB */
  RAM_D2 (rwx)    : ORIGIN = 0x30000000, LENGTH = 288K

  /* AHB SRAM (D3 domain) - 64KB */
  /* SRAM4: Used for DMA buffers in D3 domain */
  RAM_D3 (rwx)    : ORIGIN = 0x38000000, LENGTH = 64K

  /* External SDRAM - 64MB (IS42S16160) */
  /* For large audio delay buffers */
  SDRAM (rwx)     : ORIGIN = 0xc0000000, LENGTH = 64M

  /* External QSPI Flash - 8MB (W25Q64JV on some models) */
  /* Used for Execute-In-Place or data storage */
  QSPIFLASH (rx)  : ORIGIN = 0x90000000, LENGTH = 8M
}

/* Region alias required by cortex-m-rt */
/* Maps generic RAM to DTCMRAM (fast tightly-coupled memory) */
REGION_ALIAS("RAM", DTCMRAM);

/* Define custom sections for different memory regions */
SECTIONS
{
  /* SRAM1 BSS section for DMA and peripheral buffers */
  .sram1_bss (NOLOAD) : ALIGN(4)
  {
    *(.sram1_bss .sram1_bss.*);
    . = ALIGN(4);
  } > RAM_D2

  /* SDRAM BSS section for large audio buffers */
  .sdram_bss (NOLOAD) : ALIGN(4)
  {
    *(.sdram_bss .sdram_bss.*);
    . = ALIGN(4);
  } > SDRAM
}
