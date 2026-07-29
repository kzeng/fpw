/*******************************************************************************
*                   
* File Name:  memoryMap.h
*                              
* Description: Defines the flash memory map for the ADuCM430 DR8
* 
*  !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
*  !!                                                                         !!
*  !! IMPORTANT: The memory regions in defined in this file must be kept in   !!
*  !!            sync with the memory regions defined in the linker command   !!
*  !!            file. Any changes in this file require checking and updating !!
*  !!            the latter.                                                  !!
*  !!                                                                         !!
*  !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
*
* Revision History: 
*
*******************************************************************************/

#ifndef __MEMORY_MAP_H__
#define __MEMORY_MAP_H__

// *****************************************************************************
// *****************************************************************************
//
// NOTES:
// offset from FLASH_STRTADDR(STM32: Non-secure:0x8000000, Secure:0xC000000,  ADuCM430: 0x00000000)
// The flash sector size on the ADuCM430 is 4K (0x1000 bytes). These are denoted 
// by the "=================" delimiter in the diagram below 
//
// *****************************************************************************
// *****************************************************************************
// offset +====================+      offset    +====================+
//           FLASH 0                             FLASH 1
// x00000 +====================+      x100000  +====================+
//        |    GBOOT - VIC     |               |                    |
// x00250 +  -  -  -  -  -  -  +      x100250  + - - - - - - - - - -+
//        |  FW_VER_STRING     |               |  Reserved          |
// x00290 +- - - - - - - - - - +      x100290  + - - - - - - -      +
//        |  Reserved          |               |  Reserved          |
// x00400 +- - - - - - - - - - +      x100400  +- - - - - - - - - - +
//        |    CRC32 Table A   |               |    CRC32 Table B   |
// x00800 |-  -  -  -  -  -  - |      x100800  +  -  -  -  -  -  -  +  
//        |    CRC16 Table A   |               |    CRC16 Table B   |
// x00a00 +- - - - - - - - - - +      x100a00  +- - - - - - - - - - +
//        |  BOOT_CODE_REGION  |               |  BOOT_CODE_REGION  |
//        |  BOOT_CODE_REGION  |               |  BOOT_CODE_REGION  |
//        |no Signature for stm|               |no Signature for stm|
// x01000 +====================+      x101000  +====================+
//        |  BOOT_CODE_REGION  |               |  BOOT_CODE_REGION  |
// x02000 +====================+      x102000  +====================+
//        |  HW Cfg     254/  0|               |          254/ 64   | Image A/B shall have duplicate
// x02080 |-  -  -  -  -  -  - |      x102080  +  -  -  -  -  -  -  +
//        |  SW Cfg  1  254/  1|               |          254/ 65   | Addr A = 0x02000 + 0x80*Bk
// x02100 |-  -  -  -  -  -  - |      x102180  +  -  -  -  -  -  -  + Addr B = 0x102000 + 0x80*(Bk-128)
//        |  SW Cfg  2  254/  2|               |          254/ 66   |
// x02180 |-  -  -  -  -  -  - |      x102180  +  -  -  -  -  -  -  +
//        |  Mod Cal 1  254/  3|               |          254/ 67   |
// x02200 |-  -  -  -  -  -  - |      x102200  +  -  -  -  -  -  -  +
//        |  Mod Cal 2  254/  4|               |          254/ 68   |
// x02280 |-  -  -  -  -  -  - |      x102280  +  -  -  -  -  -  -  +
//        |  Mod Cal 3  254/  5|               |          254/ 69   |
// x02300 |-  -  -  -  -  -  - |      x102300  +  -  -  -  -  -  -  +
//        |             254/  6|               |                    |
// x02380 |-  -  -  -  -  -  - |      x102380  +  -  -  -  -  -  -  +
//        |             254/  7|               |                    |
// x02400 | = = = = = = = = =  |      x102400  | = = = = = = = = =  |
//        |             254/  8|               |          254/ 72   |
//        |  Low Mem Default   |               |                    |
// x02480 |-  -  -  -  -  -  - |      x102480  +  -  -  -  -  -  -  +  
//        |             254/  9|               |          254/ 73   |  
//        |  Page 00 Default   |               |                    |  
// x02500 |-  -  -  -  -  -  - |      x102500  +  -  -  -  -  -  -  +  
//        |             254/ 10|               |          254/ 74   |  
//        |  Page 01 Default   |               |                    |  
// x02580 |-  -  -  -  -  -  - |      x102580  +  -  -  -  -  -  -  +  
//        |             254/ 11|               |          254/ 75   |
//        |  Page 02 Default   |               |                    |
// x02600 |-  -  -  -  -  -  - |      x102600  +  -  -  -  -  -  -  +  
//        |             254/ 12|               |          254/ 76   | note: Skip Pg3 UsrEE
//        |  Page 04 Default   |               |                    |  
// x02680 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 13|               |          254/ 77   |  
//        |  Page 05 Default   |               |                    |  
// x02700 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 14|               |          254/ 78   |  
//        |  Page 06           |               |                    |  
// x02780 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 15|               |          254/ 79   |  
//        |  Page 07           |               |                    |  
// x02800 | = = = = = = = = =  |      x102800  | = = = = = = = = =  |
//        |             254/ 16|               |          254/ 80   |  
//        |  Page 08           |               |                    |  
// x02880 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 17|               |          254/ 81   |  
//        |  Page 09           |               |                    |  
// x02900 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 18|               |          254/ 82   |  
//        |  Page 10           |               |                    |  
// x02980 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 19|               |          254/ 83   |  
//        |  Page 11 (0xB)     |               |                    |  
// x02A00 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 20|               |          254/ 84   |  
//        |  Page 12 (0xC)     |               |                    |  
// x02A80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 21|               |          254/ 85   |  
//        |  Page 13           |               |                    |  
// x02B00 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 22|               |          254/ 86   |  
//        |  Page 14           |               |                    |  
// x02B80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 23|               |          254/ 87   |  
//        |  Page 15           |               |                    |  
// x02C00 | = = = = = = = = =  |      x102C00  | = = = = = = = = =  |
//        |             254/ 24|               |          254/ 88   |  
//        |  Page 16 (10h)     |               |                    |  
// x02C80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 25|               +          254/ 89   +  
//        |  Page 17 (11h)     |               |                    |  
// x02D00 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 26|               +          254/ 90   +  
//        |  Page 18 (12h)     |               |                    |  
// x02D80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 27|               +          254/ 91   +  
//        |  Page 19 (13h)     |               |                    |  
// x02E00 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 28|               +          254/ 92   +  
//        |  Page 20 (14h)     |               |                    |  
// x02E80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 29|               +          254/ 93   +  
//        |  Page 21 (15h)     |               |                    |  
// x02F00 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 30|               +          254/ 94   +  
//        |  Page 22 (16h)     |               |                    |  
// x02F80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 31|               +          254/ 95   +  
//        |  Page 23 (17h)     |               |                    |  
// x03000 +====================+      x103000  +====================+
//        |             254/ 32|               +          254/ 96   +  
//        |  Page 24 (18h)     |               |                    |  
// x03080 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 33|               +          254/ 97   +  
//        |  Page 25 (19h)     |               |                    |  
// x03100 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 34|               +          254/ 98   +  
//        |  Page 26 (1Ah)     |               |                    |  
// x03180 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 35|               +          254/ 99   +  
//        |  Page 27 (1Bh)     |               |                    |  
// x03200 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 36|               +          254/100   +  
//        |  Page 28 (1Ch)     |               |                    |  
// x03280 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 37|               +          254/101   +  
//        |  Page 29 (1Dh)     |               |                    |  
// x03300 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 38|               +          254/102   +  
//        |  Page 30 (1Eh)     |               |                    |  
// x03380 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |             254/ 39|               +          254/103   +  
//        |  Page 31 (1Fh)     |               |                    |  
// x03400 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 20h    254/ 40|               +          254/104   +  
//        |  Page 32 (20h)     |               |                    |  
// x03480 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 21h    254/ 41|               +          254/105   +  
//        |  Page 33 (21h)     |               |                    |  
// x03500 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 22h    254/ 42|               +          254/106   +  
//        |  Page 34 (22h)     |               |                    |  
// x03580 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 23h    254/ 43|               +          254/107   +  
//        |  Page 35 (23h)     |               |                    |  
// x03600 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 24h    254/ 44|               +          254/108   +  
//        |  Page 36 (24h)     |               |                    |  
// x03680 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 25h    254/ 45|               +          254/109   +  
//        |  Page 37 (25h)     |               |                    |  
// x03700 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 26h    254/ 46|               +          254/110   +  
//        |  Page 38 (26h)     |               |                    |  
// x03780 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 27h    254/ 47|               +          254/111   +  
//        |  Page 39 (27h)     |               |                    |  
// x03800 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 28h    254/ 48|               +          254/112   +  
//        |  Page 40 (28h)     |               |                    |  
// x03880 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 29h    254/ 49|               +          254/113   +  
//        |  Page 41 (29h)     |               |                    |  
// x03900 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 2Ah    254/ 50|               +          254/114   +  
//        |  Page 42 (2Ah)     |               |                    |  
// x03980 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 2Bh    254/ 51|               +          254/115   +  
//        |  Page 43 (2Bh)     |               |                    |  
// x03A00 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 2Ch    254/ 52|               +          254/116   +  
//        |  Page 44 (2Ch)     |               |                    |  
// x03A80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 2Dh    254/ 53|               +          254/117   +  
//        |  Page 45 (2Dh)     |               |                    |  
// x03B00 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 2Eh    254/ 54|               +          254/118   +  
//        |  Page 46 (2Eh)     |               |                    |  
// x03B80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |  VDM 2Fh    254/ 55|               +          254/119   +  
//        |  Page 47 (2Fh)     |               |                    |  
// x03C00 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis30h    254/ 56|               +          254/120   +  
//        |  Page 48 (30h)     |               |                    |  
// x03C80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis31h    254/ 57|               +          254/121   +  
//        |  Page 49 (31h)     |               |                    |  
// x03D00 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis32h    254/ 58|               +          254/122   +  
//        |  Page 50 (32h)     |               |                    |  
// x03D80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |      33h    254/ 59|               +          254/123   +  
//        |  Page 51 (33h)     |               |                    |  
// x03E00 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |      34h    254/ 60|               +          254/124   +  
//        |  Page 52 (34h)     |               |                    |  
// x03E80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |      35h    254/ 61|               +          254/125   +  
//        |  Page 53 (35h)     |               |                    |  
// x03F00 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |      36h    254/ 62|               +          254/126   +  
//        |  Page 54 (36h)     |               |                    |  
// x03F80 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |      37h    254/ 63|               +          254/127   +  
//        |  Page 55 (37h)     |               |                    |  
// x04000 +====================+      x104000  +====================+
//        | ccmis38h    248/  0|               +          248/128   +  
//        |  Page 56           |               |                    |  
// x04080 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis39h    248/  1|               +          248/129   +  
//        |  Page 57           |               |                    |  
// x04100 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis3Ah    248/  2|               +          248/130   +  
//        |  Page 58           |               |                    |  
// x04180 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis3Bh    248/  3|               +          248/131   +  
//        |  Page 59           |               |                    |  
// x04200 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis3Ch    248/  4|               +          248/132   +  
//        |  Page 60           |               |                    |  
// x04280 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis3Dh    248/  5|               +          248/133   +  
//        |  Page 61           |               |                    |  
// x04300 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis3Eh    248/  6|               +          248/134   +  
//        |  Page 62           |               |                    |  
// x04380 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis3Fh    248/  7|               +          248/135   +  
//        |  Page 63           |               |                    |  
// x04400 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis40h    248/  8|               +          248/136   +  
//        |  Page 64           |               |                    |  
// x04480 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis41h    248/  9|               +          248/137   +  
//        |  Page 65           |               |                    |  
// x04500 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis42h    248/ 10|               +          248/138   +  
//        |  Page 66           |               |                    |  
// x04580 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis43h    248/ 11|               +          248/139   +  
//        |  Page 67           |               |                    |  
// x04600 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        | ccmis44h    248/ 12|               +          248/140   +  
//        |  Page 68           |               |                    |  
// x04680 |-  -  -  -  -  -  - |               +  -  -  -  -  -  -  +  
//        |                    |               |                    |  
//        |                    |               |                    |  
//        |             248/ 63|               |          248/191   |
// x06000 +====================+      x106000  +====================+
//        |             248/ 64|               |          248/192   |
//        |                    |               |                    |  
//        |             248/127|               |          248/255   |
// x08000 +====================+      x108000  +====================+
//        |             250/  0|               |          250/128   |
//        |                    |               |                    |  
//        |             250/ 63|               |          250/191   |
// x0A000 +====================+      x10A000  +====================+
//        |             250/ 64|               |          250/192   |
//        |                    |               |                    |  
//        |             250/127|               |          250/255   |
// x0C000 +====================+      x10C000  +====================+
//        | BOOT_CTRL_BLK_IMGB |               | BOOT_CTRL_BLK_IMGA | 
// x0D000 +====================+      x10D000  +====================+
//        |                    |               |                    |
// x0E000 +====================+      x10E000  +====================+
//        |   IMG_A_CFG_DATA   |               |   IMG_B_CFG_DATA   |
// x0F000 +====================+      x10F000  +====================+
//        |                    |               |                    |
// x10000 +====================+      x110000  +====================+
//        |  IMAGE A - VIC     |               |   IMAGE B - VIC    |
// x10250 +  -  -  -  -  -  -  +      x110250  +  -  -  -  -  -  -  +
//        |  FW_VER_STRING     |               |   FW_VER_STRING    |
//        |   P 255 B 64.128   |               |    P 255 B 64.192  |
// x10290 +  -  -  -  -  -  -  +      x110290  +  -  -  -  -  -  -  +
//        |  CODE 7.5kbyte     |               |   CODE  7.5kbyte   |
// x12000 + = = = = = = = = = =+      x112000  + = = = = = = = = = =+
//        |                    |               |                    |
//        |  IMAGE A           |               |   IMAGE B          |
//        |                    |               |                    |
//        |  CODE SPACE        |               |   CODE SPACE       |
//        |  52    sectors     |               |   52    sectors    |
//        |  416     kbytes    |               |   416     kbytes   |
//        |                    |               |                    |
//        |  425888 bytes      |               |   425888 bytes     |
//        |  x10000 - x77F9F   |               |   x110000 - x177F9F|
//        |                    |               |                    |
// x76000 + = = = = = = = = = =+      x176000  + = = = = = = = = = =+
//        |                    |               |                    |
//        |                    |               |                    |
//        |  LAST CODE ADDR    |               |   LAST CODE ADDR   |
//        |    x77F9F          |               |    x177F9F         |
//        |                    |               |                    |
// x77FA0 +- - - - - - - - - - +      x177FA0  + - - - - - - - - -  +
//        |   Reserved (LITE)  |               |   Reserved (LITE)  |
// x77FF0 +- - - - - - - - - - +      x177FF0  + - - - - - - - - -  +
//        |TimeStampCnt 4 Bytes|               |TimeStampCnt 4 Bytes|
//        | Crc32_Calc 4 Bytes |               | Crc32_Calc 4 Bytes |
//        |(0x10000:0x77f9f)   |               |(0x110000:0x177f9f) |
// x77FF8 +- - - - - - - - - - +      x177FF8  + - - - - - - - - -  +
//        |                    |               |                    | 
// x77FFC +- - - - - - - - - - +      x177FFC  + - - - - - - - - -  +
//        |    fullImgCrc32    |               |    fullImgCrc32    |
//        |(0x10000:0x77f9f)   |               |(0x110000:0x177f9f) |
// x78000 +====================+      x178000  +====================+
//        |             252/  0|               |          252/ 64   |
//        |                    |               |                    |  
//        |             252/ 63|               |          252/127   |
// x7A000 +====================+      x17A000  +====================+
//        |  (Resv UsrEeLib)   |               |   (Resv UsrEeLib)  | 
//        |   User EEPROM      |               |    User EEPROM     |
//        |     Sector 1       |               |      Sector 1      |
// x7B000 +====================+      x17B000  +====================+
//        |  (Resv UsrEeLib)   |               |   (Resv UsrEeLib)  |
//        |   User EEPROM      |               |    User EEPROM     |
//        |     Sector 2       |               |      Sector 2      |
// x7C000 +====================+      x17C000  +====================+
//        |                    |               |                    | 
// x7D000 +====================+      x17D000  +====================+
//        |                    |               |                    | 
// x7E000 +====================+      x17E000  +====================+
//        |                    |               |                    |
//        |                    |               |                    |
// x7E080 +- - - - - - - - - - +      x17E080  + - - - - - - - - -  +
//        |  Product Page      |               |  Product Page      |
//        |  P 255 B 1         |               |  P 255 B 65        |
//        |                    |               |                    |
//        |  Minimize Erase    |               |  Duplicate Backup  |
//        |  Write Once in     |               |                    |
//        |  Production.       |               |                    |
// x7E080 +- - - - - - - - - - +      x17E080  + - - - - - - - - -  +
//        |  Dsp Img Info      |               |  Resv Inventory    |
//        |  P 255 B 2         |               |  P 255 B 66        |
// x7E100 +- - - - - - - - - - +      x17E100  + - - - - - - - - -  +
//        |  Resv Inventory    |               |  Resv Inventory    |
//        |  P 255 B 3         |               |  P 255 B 67        |
// x7E180 +- - - - - - - - - - +      x17E180  + - - - - - - - - -  +
//        |  P 255 B 4         |               |  P 255 B 68        |
// x7E200 +- - - - - - - - - - +      x17E200  + - - - - - - - - -  +
//        |                    |               |                    |
// x7E280 +- - - - - - - - - - +      x17E280  + - - - - - - - - -  +
//        |                    |               |                    |
// x7F000 +====================+      x17F000  +====================+
//        |                    |               |                    |
// x7FFC0 +- -   -   -   -   - +      x17FFC0  +-   -   -   -   -   +
//        | Key1               |               | Key1'              |
// x7FFC4 +-   -   -   -   -   +      x17FFC4  +-   -   -   -   -   +
// x7FFC8 +-   -   -   -   -   +      x17FFC8  +-   -   -   -   -   +
//        | Key2               |               | Key2'              |
// x7FFCC +-   -   -   -   -   +      x17FFCC  +-   -   -   -   -   +
// x7FFD0 +- -   -   -   -   - +      x17FFD0  +-   -   -   -   -   +
// x7FFD8 +-   -   -   -   -   +      x17FFD8  +-   -   -   -   -   +
// x7FFE0 +- -   -   -   -   - +      x17FFE0  +-   -   -   -   -   +
//        | SWD Diable         |               | SWD Diable         |
// x7FFE8 +-   -   -   -   -   +      x17FFE8  +-   -   -   -   -   +
//        | User FEEKEY        |               | User FEEKEY        |
// x7FFEC +-   -   -   -   -   +      x17FFEC  +-   -   -   -   -   +
//        | Reserved           |               | Reserved           |
// x7FFF0 +-   -   -   -   -   +      x17FFF0  +-   -   -   -   -   +
//        | Signature,WrProt   |               | Signature,WrProt   |
// x7FFF4 +-   -   -   -   -   +      x17FFF4  +-   -   -   -   -   +
//        | Reserved           |               | Reserved           |
// x7FFF8 +-   -   -   -   -   +      x17FFF8  +-   -   -   -   -   +
//        | Reserved           |               | Reserved           |
// x7FFFC +-   -   -   -   -   +      x17FFFC  +-   -   -   -   -   +
//        | Signature CRC32         |          | Signature CRC32    |
// x80000 +-   -   -   -   -   +      x180000  +-   -   -   -   -   +
//        | Reserved 512k      |               | Reserved 512k      | 
//x100000 +====================+      x200000  +====================+
//                                             | ADI Factory Code   |
//                                             | Information Space  |
//                                             | (16 kbytes)        |
//                                    x104000  +====================+
//
// *****************************************************************************
// *****************************************************************************
// Release Binary mem allocation Description(Xianqing Shao, 2025/09/11), NOTE that ReleaseBinary copy data from mcuflash in range[IMG COPY BEGIN: IMG COPY END]

// Corresponding MCU FLASH                                              +       ReleaseBinary(CMIS upgrade) via imgAr.exe                                                                        
// x0C000 +====================+      x10C000  +====================+              
//        |                    |               |                    |     x00000  +====================+       x6A100  +====================+       xD4200  +====================+     
//        | BOOT_CTRL_BLK_IMGB |               | BOOT_CTRL_BLK_IMGA |             |   Header_ImgA      |               |   Header_ImgB      |               |   Header_DSP_ImgA  |   
// x0E000 +====================+      x10E000  +====================+     x00100  +====================+       x6A200  +====================+       xD4300  +====================+   
//        |   IMG_A_CFG_DATA   |               |   IMG_B_CFG_DATA   |             |pad ffh ifnot in Hex|               |pad ffh ifnot in Hex|               |        ...         |   
// x10000 +====================+      x110000  +====================+     x02100  +====================+       x6C200  +====================+               |        ...         |    
//        |  IMAGE A - VIC     |               |   IMAGE B - VIC    |             |   IMG COPY BEGIN   |               |   IMG COPY BEGIN   |               |    DSP IMG DATA    |     
// x10250 +  -  -  -  -  -  -  +      x110250  +  -  -  -  -  -  -  +             |fullImgCrc32 calcBeg|               |fullImgCrc32 calcBeg|               |  pad ffh if needed |   
//        |  FW_VER_STRING     |               |   FW_VER_STRING    |             |imgCrc32 calcBeg    |               |imgCrc32 calcBeg    |       ImgAEnd +====================+    
//        |   P 255 B 64.128   |               |    P 255 B 64.192  |
// x10290 +  -  -  -  -  -  -  +      x110290  +  -  -  -  -  -  -  + 
//        |  CODE 7.5kbyte     |               |   CODE  7.5kbyte   |
// x12000 + = = = = = = = = = =+      x112000  + = = = = = = = = = =+
//        |                    |               |                    |
//        |  IMAGE A           |               |   IMAGE B          |
//        |                    |               |                    |
//        |  CODE SPACE        |               |   CODE SPACE       | 
//        |  52 sectors        |               |   52 sectors       | 
//        |  416 kbytes        |               |   416 kbytes       |
//        |                    |               |                    |
//        |  425788 bytes      |               |   425788 bytes     |
//        |  x10000 - x77F9F   |               |   x110000 - x117F9F|
//        |                    |               |                    |
// x76000 + = = = = = = = = = =+      x176000  + = = = = = = = = = =+
//        |                    |               |                    |
//        |                    |               |                    |
//        |  LAST CODE ADDR    |               |   LAST CODE ADDR   |
//        |    x77F9F          |               |    x177F9F         |
//        |                    |               |                    |             |fullImgCrc32 calcEnd|               |fullImgCrc32 calcEnd|
// x77FA0 +- - - - - - - - - - +      x177FA0  + - - - - - - - - -  +     x6A0A0  +- - - - - - - - - - +       xD41A0  + - - - - - - - - -  +
//        |   Reserved (LITE)  |               |   Reserved (LITE)  |             |                    |               |                    |
// x77FF0 +- - - - - - - - - - +      x177FF0  + - - - - - - - - -  +     x6A0F0  +  -  -  -  -  -  -  +       xD41F0  +  -  -  -  -  -  -  +
//        |TimeStampCnt 4 Bytes|               |TimeStampCnt 4 Bytes|             |                    |               |                    |
//        | Crc32_Calc 4 Bytes |               | Crc32_Calc 4 Bytes |             |                    |               |                    |
//        |(0x10000:0x77f9f)   |               |(0x110000:0x177f9f) |             |                    |               |                    |
//        |                    |               |                    |             |                    |               |                    |
//        |                    |               |                    |             |   IMG COPY END     |               |   IMG COPY END     | 
// x77FF8 +- - - - - - - - - - +      x177FF8  + - - - - - - - - -  +     x6A0F8  +- - - - - - - - - - +       xD41F8  + - - - - - - - - -  +
//        |                    |               |                    |             |      imgCrc32      |               |     imgCrc32       | 
//        |                    |               |                    |             | except erased tail |               | except erased tail |
// x77FFC +- - - - - - - - - - +      x177FFC  + - - - - - - - - -  +     x6A0FC  +- - - - - - - - - - +       xD41FC  + - - - - - - - - -  +
//        |    fullImgCrc32    |               |    fullImgCrc32    |             |    fullImgCrc32    |               |    fullImgCrc32    |
//        |(0x10000:0x77f9f)   |               |(0x110000:0x177f9f) |             |(0x10000:0x77f9f)   |               |(0x110000:0x177f9f) |
// x78000 +====================+      x178000  +====================+     x6A100  +====================+       xD4200  +====================+


// *****************************************************************************
// *****************************************************************************
//
// SRAM, 
//
// NOTE: For QSFPDD we are planning to use MODE0
//
//                
//                |-----------|
//                |  4K cache |
//                +-----------+
//  x1000 0000    +-----------+
//                |           |
//    SRAM BANK0  | 16K ISRAM |
//                |           |
//  x1000 4000    +-----------+
//                |           |
//                    ...
//                |           |
//  x2000 0000    +-----------+
//                |           |
//    SRAM BANK1  | 32K DSRAM |
//                |           |
//  x2000 8000    +-----------+
//  x2000 8000    +-----------+
//                |           |
//    SRAM BANK2  | 16K DSRAM |
//                |           |
//  x2000 C000    +-----------+
//
//                
//                

// RAM
#define IRAM_ADDR_BASE                  0x20040000L   // skip SRAM1 for its non-ECC   
#define RAM_SIZE_                       0x00050000L   // ECC SIZE is 16k+48k+256k=320k, whole RAM is 640k
                                                      
#define NUM_VECTOR_TABLE_ENTRIES        147
#define VECTOR_TABLE_SIZE_BYTES         (NUM_VECTOR_TABLE_ENTRIES * 4) // 588 = 0x24c = about 0x280
#define SRAM_VECTOR_TABLE               IRAM_ADDR_BASE   // SRAM VIC table.
#define SRAM_VECTOR_TABLE_SIZE          0x250 

#define SRAM_BOOTAPPL_SHARED_PRAM       (SRAM_VECTOR_TABLE + SRAM_VECTOR_TABLE_SIZE)   
#define SRAM_BOOTAPPL_SHARED_PRAM_SIZE  0x30 

#define SRAM_PRAM_BASE                  (IRAM_ADDR_BASE + 0x280L)   // 512 bytes
#define SRAM_PRAM_SIZE                  0x200         // 512 bytes

// #define SRAM_ADDR_BASE                  0x20004000L   // ADuCM430

// FLASH
#define FLASH_STRTADDR                  0x08000000L // 0x00000000L ADuCM430
#define FLASH_SIZE_                     0x100000    // size per image, Flash0 = 1MB, Flash1 = 1MB

#define FLASH_0_START                   (FLASH_STRTADDR + 0x00000000L)
#define FLASH_0_END                     (FLASH_STRTADDR + (FLASH_SIZE_ - 1))
#define FLASH_1_START                   (FLASH_STRTADDR + FLASH_SIZE_)
#define FLASH_1_END                     (FLASH_STRTADDR + (2*FLASH_SIZE_ - 1))

#define QSFP_CRC32_TABLE_A               (FLASH_STRTADDR + 0x00400L) // 1024 bytes, NOTE: never forget bracket for macro define with operator
#define QSFP_CRC16_TABLE_A               (FLASH_STRTADDR + 0x00800L) //  512 bytes
#define QSFP_CRC32_TABLE_B               (FLASH_STRTADDR + FLASH_SIZE_ + 0x00400L) // 1024 bytes
#define QSFP_CRC16_TABLE_B               (FLASH_STRTADDR + FLASH_SIZE_ + 0x00800L) //  512 bytes

/*
// BOOT
*/
#define BOOT_IMG_VECTOR                 (FLASH_STRTADDR + 0x00000000L)
#define BOOT_IMG_FW_VER_STRING          (FLASH_STRTADDR + 0x00000250L)
#define BOOT_IMG_ADDR_START             BOOT_IMG_VECTOR
#define BOOT_IMG_ADDR_END               (FLASH_STRTADDR + 0x00001FFFL)

/*
// BOOT/APPL shared flash
//
//
//
//
// However: 
//   IMAGE A (lower bank) will only erase BOOT_CTRL_IMGA (upper bank).
//   IMAGE B will only erase BOOT_CTRL_IMGB.
//
// From experience writing the minimum size of words to the
// same bank will stop the CPU (on the 70xx/71xx) for < 25 usec.
// Erasing a flash block could take up to 11 msec on average.
// 
// ADuCM430... datasheet list typical times below.
//  - timing to write 64/72 bits (minimum size) is 46 usec
//  -                                              20 usec sequential writes
//  - erase page 11 msec.
*/
// ************************************************
// Image A related definitions
// ************************************************

#define BOOT_CTRL_BLK_IMGA              (FLASH_STRTADDR + (FLASH_SIZE_ + 0x0000C000L))
#define APPL_IMGA_FW_CFG_DATA           (FLASH_STRTADDR + 0x0000E000L)
#define APPL_IMGA_VECTOR                (FLASH_STRTADDR + 0x00010000L)
#define APPL_IMGA_FW_VER_STRING         (FLASH_STRTADDR + 0x00010250L)
#define APPL_IMGA_END                   (FLASH_STRTADDR + 0x00077F9FL)  //xianqing, adjust to 0x00077F9FL in future, no need to reserve so large if no DSP img in mcu flash

#define APPL_IMGA_TIMESTAMP_COUNTER     (FLASH_STRTADDR + 0x00077FF0L)
#define APPL_IMGA_VALID_CRC             (FLASH_STRTADDR + 0x00077FF4L)
#define APPL_IMGA_CRC32                 (FLASH_STRTADDR + 0x00077FFCL)
#define APPL_IMGA_CRC32_END             (FLASH_STRTADDR + 0x00078000L)

#define EEPROM_IMGA_START0              (FLASH_STRTADDR + 0x0007A000L) 
#define EEPROM_IMGA_START1              (FLASH_STRTADDR + 0x0007C000L) 

#define NVR_START_ADDR1_A                (FLASH_STRTADDR + 0x02000L)   // NVR Range1: page 254,248,250
#define NVR_END_ADDR1_A                  (FLASH_STRTADDR + 0x0E000L)   // NVR Range1: page 254,248,250
#define NVR_START_ADDR2_A                (FLASH_STRTADDR + 0x78000L)   // NVR Range2: page 252
#define NVR_END_ADDR2_A                  (FLASH_STRTADDR + 0x7A000L)   // NVR Range2: page 252
#define NVR_START_ADDR3_A                (FLASH_STRTADDR + 0x7E000L)   // NVR Range3: page 255
#define NVR_END_ADDR3_A                  (FLASH_STRTADDR + 0x7F000L)   // NVR Range3: page 255

#define NVR_PG254_BASEADDR_A(bank)       (FLASH_STRTADDR + (0x02000 + (128*(bank))))
#define NVR_PG248_BASEADDR_A(bank)       (FLASH_STRTADDR + (0x04000 + (128*(bank))))
#define NVR_PG250_BASEADDR_A(bank)       (FLASH_STRTADDR + (0x08000 + (128*(bank))))
#define NVR_PG252_BASEADDR_A(bank)       (FLASH_STRTADDR + (0x78000 + (128*(bank))))
#define NVR_PG255_BASEADDR_A(bank)       (FLASH_STRTADDR + (0x7e000 + (128*(bank))))

// ************************************************
// Image B related definitions
// ************************************************
#define BOOT_CTRL_BLK_IMGB              (FLASH_STRTADDR + 0x0000C000L)
#define APPL_IMGB_FW_CFG_DATA           (FLASH_SIZE_ + APPL_IMGA_FW_CFG_DATA)
#define APPL_IMGB_VECTOR                (FLASH_SIZE_ + APPL_IMGA_VECTOR)
#define APPL_IMGB_FW_VER_STRING         (FLASH_SIZE_ + APPL_IMGA_FW_VER_STRING)
#define APPL_IMGB_END                   (FLASH_SIZE_ + APPL_IMGA_END)

#define APPL_IMGB_TIMESTAMP_COUNTER     (FLASH_SIZE_ + APPL_IMGA_TIMESTAMP_COUNTER)
#define APPL_IMGB_VALID_CRC             (FLASH_SIZE_ + APPL_IMGA_VALID_CRC)
#define APPL_IMGB_CRC32                 (FLASH_SIZE_ + APPL_IMGA_CRC32)
#define APPL_IMGB_CRC32_END             (FLASH_SIZE_ + APPL_IMGA_CRC32_END) //0x0001F8000L 

#define EEPROM_IMGB_START0              (FLASH_SIZE_ + EEPROM_IMGA_START0)
#define EEPROM_IMGB_START1              (FLASH_SIZE_ + EEPROM_IMGA_START1)   

#define NVR_START_ADDR1_B                (FLASH_SIZE_ + NVR_START_ADDR1_A)      // NVR Range1: page 254,248,250
#define NVR_END_ADDR1_B                  (FLASH_SIZE_ + NVR_END_ADDR1_A)       // NVR Range1: page 254,248,250
#define NVR_START_ADDR2_B                (FLASH_SIZE_ + NVR_START_ADDR2_A)      // NVR Range2: page 252
#define NVR_END_ADDR2_B                  (FLASH_SIZE_ + NVR_END_ADDR2_A)       // NVR Range2: page 252
#define NVR_START_ADDR3_B                (FLASH_SIZE_ + NVR_START_ADDR3_A)      // NVR Range3: page 255
#define NVR_END_ADDR3_B                  (FLASH_SIZE_ + NVR_END_ADDR3_A)        // NVR Range3: page 255

#define NVR_PG254_BASEADDR_B(bank)       (FLASH_SIZE_ + NVR_PG254_BASEADDR_A(bank)) // FLASH_STRTADDR + (FLASH_SIZE_ + 0x02000 + (128*(bank)))
#define NVR_PG248_BASEADDR_B(bank)       (FLASH_SIZE_ + NVR_PG248_BASEADDR_A(bank)) // FLASH_STRTADDR + (FLASH_SIZE_ + 0x04000 + (128*(bank)))
#define NVR_PG250_BASEADDR_B(bank)       (FLASH_SIZE_ + NVR_PG250_BASEADDR_A(bank)) // FLASH_STRTADDR + (FLASH_SIZE_ + 0x08000 + (128*(bank)))
#define NVR_PG252_BASEADDR_B(bank)       (FLASH_SIZE_ + NVR_PG252_BASEADDR_A(bank)) // FLASH_STRTADDR + (FLASH_SIZE_ + 0xf8000 + (128*(bank)))
#define NVR_PG255_BASEADDR_B(bank)       (FLASH_SIZE_ + NVR_PG255_BASEADDR_A(bank)) // FLASH_STRTADDR + (FLASH_SIZE_ + 0xfe000 + (128*(bank)))

#define QSFP_LOW_MEM_MIRROR_MEM_ADDR      (FLASH_STRTADDR + 0x02400L)
#define QSFP_PAGE_00_MIRROR_MEM_ADDR      (FLASH_STRTADDR + 0x02480L)
#define QSFP_PAGE_01_MIRROR_MEM_ADDR      (FLASH_STRTADDR + 0x02500L)
#define QSFP_PAGE_02_MIRROR_MEM_ADDR      (FLASH_STRTADDR + 0x02580L)
#define QSFP_PAGE_04_MIRROR_MEM_ADDR      (FLASH_STRTADDR + 0x02600L)

#define QSFP_PAGE_16_MIRROR_MEM_ADDR      (FLASH_STRTADDR + 0x02C00L)
#define QSFP_PAGE_17_MIRROR_MEM_ADDR      (FLASH_STRTADDR + 0x02C80L)
#define QSFP_PAGE_18_MIRROR_MEM_ADDR      (FLASH_STRTADDR + 0x02D00L)
#define QSFP_PAGE_19_MIRROR_MEM_ADDR      (FLASH_STRTADDR + 0x02D80L)
#define QSFP_PAGE_20_MIRROR_MEM_ADDR      (FLASH_STRTADDR + 0x02E00L)

// VDM ROM Pages
// #define QSFP_PAGE_20H_MIRROR_MEM_ADDR       FLASH_STRTADDR + 0x03400L
// #define QSFP_PAGE_28H_MIRROR_MEM_ADDR       FLASH_STRTADDR + 0x03800L

// C-CMIS ROM Pages
#define QSFP_PAGE_41H_MIRROR_MEM_ADDR       (FLASH_STRTADDR + 0x04480L)
#define QSFP_PAGE_42H_MIRROR_MEM_ADDR       (FLASH_STRTADDR + 0x04500L)
#define QSFP_PAGE_43H_MIRROR_MEM_ADDR       (FLASH_STRTADDR + 0x04580L)
#define QSFP_PAGE_44H_MIRROR_MEM_ADDR       (FLASH_STRTADDR + 0x03600L)

// #define QSFP_EEPROM_PAGE_MEM_ADDR          FLASH_STRTADDR + 0xF8000L
// #define QSFP_PASSWORD_PAGE_MEM_ADDR        FLASH_STRTADDR + 0xFA000L
// 
#define QSFP_CONFIG_250_00_MEM_ADDR           (FLASH_STRTADDR + 0x08000L)

//FW RELEASE ALLOCATION
#define REL_HDR_SIZE            0x100L  // header
#define REL_IMGA_SIZE           0x6A000L  // without header, need be multiple of sector size


#define DSPFW_MCUFLASH_STRTADDR_P1  (FLASH_STRTADDR + 0x00080000)  //DSP internal MCUFLASH IMGA part1 start address
#define DSPFW_MCUFLASH_LEN_P1       0x80000  //DSP internal MCUFLASH IMGA length part1, 512k
#define DSPFW_MCUFLASH_STRTADDR_P2  (DSPFW_MCUFLASH_STRTADDR_P1 + FLASH_SIZE_)  //DSP internal MCUFLASH IMGA part2 start address
#define DSPFW_MCUFLASH_LEN_P2       0x13000  //DSP internal MCUFLASH IMGA length part2, 12k

#define DSPFW_MCUFLASH_STRTADDR_A   DSPFW_MCUFLASH_STRTADDR_P1  //DSP internal MCUFLASH IMGA start address
#define DSPFW_MCUFLASH_LEN_A        (DSPFW_MCUFLASH_LEN_P1 + DSPFW_MCUFLASH_LEN_P2)  //DSP internal MCUFLASH IMGA length, 512k(previous), 524k(since fw D0011020).


//SPI-FLASH MEMORY
#define MAX_DSP_BIN_SIZE            0x100000  //SIAN2: 0x60000, SIAN3: 0x83000, add margin for future DSP code growth 0x80000 + 0x8000 = 0x88000
#define DSP_IMGA                    0
#define DSP_IMGB                    1
#define DSP_KEY1_ADDR               0x5000   // TBD  in sian3
#define DSP_KEY2_ADDR               0x1F9000  // TBD  in sian3
#define DSP_IMGB_STRTADDR_OFST      0x7000   // IMGB need skip 0x7000 bytes in bin file
#define APPL_DSP_IMGA_ADDR          0x0
#define APPL_DSP_IMGA_P1_ADDR       0x0      // Image A part1 start addr in spi-flash, length is 0x80000, same as DSP internal MCUFLASH IMGA part1
#define APPL_DSP_IMGA_P2_ADDR       0x80000  // Image A part2 start addr in spi-flash, length is 0x3000, same as DSP internal MCUFLASH IMGA part2
#define APPL_DSP_IMGB_ADDR          0x100000  // TBD  in sian3
#define APPL_DSP_IMGB_P1_ADDR       0x100000    // Image A part1 start addr in spi-flash, length is 0x80000, same as DSP internal MCUFLASH IMGA part1
#define APPL_DSP_IMGB_P2_ADDR       (0x180000 - DSP_IMGB_STRTADDR_OFST)   // Image A part2 start addr in spi-flash, length is 0x3000, same as DSP internal MCUFLASH IMGA part2



#endif
