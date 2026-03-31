/* STM32H743 baseline memory map placeholder.
 * This file is present now so the repository has an explicit memory-layout
 * anchor before embedded bring-up starts. Final values must be confirmed
 * against the exact package and linker strategy chosen for production.
 */

MEMORY
{
  FLASH     : ORIGIN = 0x08000000, LENGTH = 2048K
  ITCMRAM   : ORIGIN = 0x00000000, LENGTH = 64K
  DTCMRAM   : ORIGIN = 0x20000000, LENGTH = 128K
  RAM_D1    : ORIGIN = 0x24000000, LENGTH = 512K
  RAM_D2    : ORIGIN = 0x30000000, LENGTH = 288K
  RAM_D3    : ORIGIN = 0x38000000, LENGTH = 64K
}
