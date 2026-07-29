/*******************************************************************************
*                   
* File Name:  fileHdr.h
*                              
* Description: This file consist of file headers to separate various FW loads.
*
* Revision History: 
*
*******************************************************************************/

#ifndef __FILE_HDR_H_
#define __FILE_HDR_H_


#define FILETYPE_IMG_A      0xd8
#define FILETYPE_IMG_B      0xd9
#define FILETYPE_DSP_SIAN2  0xdb
#define FILETYPE_DSP_SIAN2_IMG_A  0xdb
#define FILETYPE_DSP_SIAN2_IMG_B  0xdc
#define FILETYPE_DSP_SIAN3  FILETYPE_DSP_SIAN2   //to be compatible with previous release
#define FILETYPE_NVR_REG    0xdd  // for NVR register programming

#define PRODCODE_QSFPDD_400G_ZR         0xD4
#define PRODCODE_QSFPDD_800G            0xD8
#define PRODCODE_QSFPDD_STROSA          0x50
#define PRODCODE_1600_DR8_BRCM          0x16    //BRCM SIAN2  
#define PRODCODE_1600_DR8_SIAN2_BERT    0x17    //BRCM SIAN2 BERT
#define PRODCODE_1600_DR8_ARA           0x18    //MRVL ARA, Trefork
#define PRODCODE_1600_DR8_SIAN3         0x19    //BRCM SIAN3

#define ENCRYPT_NONE 0
#define ENCRYPT_POORMAN_U32   0xd
#define ENCRYPT_POORMAN_U128  0xf

#define PROD_TYPE_EXT_UNDEF    0
#define PROD_TYPE_EXT_IHS      1
#define PROD_TYPE_EXT_RHS      2

typedef struct
{
    // - -  0 - - 
    char  lite[4];

    char  fileType;     // 0xd8:IMG-A 0xd9:IMG-B  0xdb:DSP_SIAN2_IMG_A, 0xdc:DSP_SIAN2_IMG_B  NOTE BRCM DSP image diff for A/B

    char  encType:4;         // Encryption Type , 0xd: poor-man's encryption, 0: non-encrypted
    char  flag_nvrAltBase :1;     //if '1' then NVR corresponding flash sector will be updated based on read-back data of alternate image bank(active image bank).
    char  flag_skipDspHdrImgA:1;  // if '1' then for dsp image, fw will exclude first 0x7000 bytes(SIAN3 DSP FW IMAGE HEADER) during ImageA download.
    char  ignoreNvrMskRule:1; // if '1' then ignore NVR rule check, only for NVR img
    char  nta:1;             // if '1' then NTA capable (non traffic affecting)

    unsigned char prodCode;   // 0xD4 (QSFP-DD 400G ZR),0xd8(QSFPDD OSFP 800G),0x16 - 1600 DR8 BRCM, 0x17 - 1600 DR8 BRCM-BERT
    unsigned char hdrChksum;

    // - -  8 - - 
    unsigned char  major;    // Major
    unsigned char  minor;    // Minor
    unsigned short build;    // Build

    // - - 12  - - 
    unsigned int   datalen;  // actual datalen (excludes header/padding 0xff), used to calc qddTrl->imgCrc32.

    // - - 16  - - 
    char  date[8];  // YYYYMMDD
    char  time[8];  // HH:MM:SS

    // - - 32  - - 
    char  filename[32];

    // - - 64  - - 
    unsigned int   totlen;   // total len(with padded with 0xff, exclude header) to checksum calc(qddTrl->fullImgCrc).

    // - - 68  - - 
    unsigned int   imglen;  // image len(exclude header) to be wrote to flash.
    
    unsigned int   imgCrc32;  // fullImgCrc based on totlen.
    // - - 76  - - 
    unsigned int   encStrtAddr;
    // - - 80  - -
    unsigned int   encLen;
    // - - 84  - -
    unsigned int   imgCrc32_dspB;  // addtional crc32 for SIAN3 dsp imgB
    // - - 88  - -
    unsigned char  nvr_page;
    // - - 89  - -  
    unsigned char  nvr_bank;
    // - - 90  - -
    unsigned char  prod_type_ext; // extend prod type, 0(default) means not-specified, 1: only for IHS, 2: only for RHS
    // - - 91  - -
    unsigned char  nvr_reg_addr;
    // - - 92  - -
    char  resv[36];

    // - - 128 - - 
    char  encKey[128]; // Resv for keys... for encoded / encrypted application code.
} fileHdr_t;


typedef struct
{
    // - - 

    unsigned int    timestampCnt;     // on command complete FW will update these after checks.
    unsigned int    crcVal;     // on command complete FW will update these after checks.
    
    unsigned int    imgCrc32;   // 
    unsigned int    fullImgCrc;
} fileTrl_t;

#endif
