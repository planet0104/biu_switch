MEMORY
{
    /* Adafruit / Nice!Nano V2 UF2 Bootloader + S140 6.1.1 */
    FLASH : ORIGIN = 0x00026000, LENGTH = 872K
    /* 预留 SoftDevice RAM 区（S140 6.1.1 最低约 0x22B0），避免与 Bootloader 冲突 */
    RAM   : ORIGIN = 0x200022B0, LENGTH = 256K - 0x22B0
}
