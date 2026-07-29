// mergeIntelHex.cpp : Defines the entry point for the console application.
//

#include "stdafx.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "hexFileLib.h"
#include "fileHdr.h"
#include "memoryMap.h"

fileHdr_t   qddHdr;

int imgStrtAddr;
int fleStrtAddr;
int imgStopAddr;
int imgLen;


#define STROSA   0


int quietMode = 0;
int debug = 0;



unsigned int calcCrc32( unsigned char *data, unsigned int len, unsigned int crc );

static unsigned char  imageAddr [65536*16*4]; // 1024k space... 4096 k for DSP.



// -----------------------------------------------------------------------------
// -----------------------------------------------------------------------------
void dumpImg( int offset, int len )
{
    int i;

    for (i=0;i<len;i++)
    {
        fprintf(stderr,"%#04x ", imageAddr[offset+i] );
        if ((i%8)==7  )  fprintf(stderr, " :: " );
        if ((i%16)==15)  fprintf(stderr, "\n" );
    }
    fprintf( stderr, "\n\n");
}

// -----------------------------------------------------------------------------
// Read all hex data into a buffer.
// Return:
//  - number of lines
// -----------------------------------------------------------------------------
int  hexReadData( FILE *fp )
{
    int lines;
    int j, rc;
    int size;
    unsigned int addrHiWord;
    unsigned int addr;
    //int extAddr;
    int hexDataType;
    char hexline[512];
    unsigned char *buf;

    fseek(  fp, 0, SEEK_SET );
    lines = 0;

    addrHiWord = 0x0000;
    buf        = &imageAddr[0];

    while (1)
    {
      rc = fscanf_s( fp, "%[:0-9A-F]\n", hexline,512 );

	  // rc = 0;
      if ((rc==0) || (rc==EOF)) return lines;

	  
      if ( (strcmp(hexline, ":00000001FF")!=0) &&
           (strncmp(hexline, ":04", 3)!=0) )
      {
      }
      lines++;
	  

      size   = hexToByte( &hexline[1] );
      addr   = hexToByte( &hexline[3] );
      addr <<= 8;
      addr  |= hexToByte( &hexline[5] );

      hexDataType = hexToByte( &hexline[7] );

      if (size==0) 
      {
        return lines;
      }

      switch (hexDataType)
      {
        // 0x00 - Data
        case 0x00:
        {
            // printf("AddrHiWord = %#06x Addr = %#06x\n", addrHiWord, addr);
            for (j = 0;j < size;++j)
            {
                buf[addrHiWord + addr + j - FLASH_STRTADDR] = hexToByte(&hexline[9 + j * 2]); //xqshao, imageAddr store data with offset=FLASH_STRTADDR
                // printf("%02x ", buf[addrHiWord+addr+j]);
            }



            // printf("\n");
            break;
        }


        // Safety.  Should have terminated above.
        // 0x01 - ":00000001FF" end of file.
        case 0x01:
          return lines;


        case 0x02:
            addrHiWord = hexToByte(&hexline[9]);
            addrHiWord <<= 8;
            addrHiWord |= hexToByte(&hexline[11]);
            addrHiWord <<= 4; //4

            /*
            if (addrHiWord == 0x2) //comment out, buffer should never be changed
            {
                buf = &imageAddr[131072]; //bug?
            }
            else
            {
                buf = &imageAddr[0];
            } */

            break;

            // 0x04 - Extended Hi Addr
        case 0x04:
            addrHiWord = hexToByte(&hexline[9]);
            addrHiWord <<= 8;
            addrHiWord |= hexToByte(&hexline[11]);
            addrHiWord <<= 16; //4

            /*
            if (addrHiWord==0x2) //comment out, buffer should never be changed
            {
              buf = &imageAddr[131072]; //bug?
            }
            else
            {
              buf = &imageAddr[0];
            } */
            break;



        // 0x05 - Ignore
        // 0x02, 0x03 - Ignore for now...
        case 0x05:
        default:
          continue;
      }
    }
    
    return lines;
}





// -----------------------------------------------------------------------------
//
// -----------------------------------------------------------------------------
unsigned short calcWordChksum( unsigned char *data, int size )
{
  unsigned short  chksum;
  unsigned char  *ucdata;
  int  i;

  ucdata = (unsigned char *) data;
  chksum = 0;

  for (i=0;i<size;++i)
  {
    chksum += *ucdata++;
  }
  return chksum;
}



char toolname[256];

// -----------------------------------------------------------------------------
//
// -----------------------------------------------------------------------------
void usage( void )
{
    fprintf( stderr, "Usage:  " );
// imgAr outFile encType fileType major.minor.build inputfile
    fprintf( stderr, "%s outFile encType fileType dateStr timeStr infile.hex\n", toolname );
    exit(1);
}




// Flags




// -----------------------------------------------------------------------------
//
// -----------------------------------------------------------------------------
int  isAddrErased( unsigned char *buf, int sectAddr, int size )
{
  int  i;
  unsigned short *psdata;

  psdata = (unsigned short *) (buf + sectAddr);

  if (size&0x1)
  {
    fprintf(stderr, "SWERR: size cannot be odd\n");
    exit(3);
  }

  for (i=0;i<size/2;++i)
  {
    if (*psdata++ != 0xFFFF) return 0;
  }
  return 1;
}



// -----------------------------------------------------------------------------
//  encodeType1(PoorMan), with unit is UINT_64(8 bytes)
// -----------------------------------------------------------------------------
void encodeType1BinFile(int isImgA, int binFileSize)
{
    unsigned int addr;
    unsigned int aPhase;
    unsigned short *pSw;
    unsigned long long *pLl;

    unsigned int strtEnc;
    unsigned int stopEnc;
    unsigned int nEncBlk;

    // Too Small..
    if (binFileSize<=(512*3)) return;

    // Don't encode 1st block and last 512..1023 bytes
    nEncBlk = (binFileSize - 512 - 512) / 512;

    // For now, start encoding from the same address if A or B
    if (isImgA) { strtEnc = 512; }
    else        { strtEnc = 512; }

    stopEnc = strtEnc + (nEncBlk*512);
    //printf("strt = %d\n", strtEnc );
    //printf("stop = %d\n", stopEnc );

    for (addr=strtEnc;addr<stopEnc;addr+=8)
    {
        pSw = (unsigned short *)     &imageAddr[addr];
        pLl = (unsigned long long *) &imageAddr[addr];

        if ((*pLl != 0xffffffffffffffff) && (*pLl != 0x0))
        {
            aPhase = (addr & 0x18)>>3;

            if ((aPhase&1)==0)
            {
                *pLl = ~*pLl;
            }
            if (aPhase)
            {
                pSw  = pSw + aPhase;
                *pSw = ~*pSw;
            }
        }
    }
}
// -----------------------------------------------------------------------------
//  encodeType1(PoorMan), with unit is UINT_32(4 bytes),  in ADuCM430, flash write unit is UINT_32(4 bytes)
// -----------------------------------------------------------------------------
void encodeType1BinFile_UINT32(unsigned int   encStrtAddr, unsigned int   encLen)
{
    unsigned int addr;
    unsigned int strtEnc = encStrtAddr;
    unsigned int stopEnc = encStrtAddr + encLen;

    for (addr=strtEnc;addr<stopEnc;addr+=4) // 4 bytes one time
    {
        unsigned char *pSw = (unsigned char *) &imageAddr[addr];
        unsigned int *pData = (unsigned int *) &imageAddr[addr];

        if(*pData == 0x0 || *pData == 0xffffffff)
        {
            continue;
        }
        unsigned short aPhase = (addr & 0x0c) >> 2;  //aPhase is 0-3
        if((aPhase&1)==0)
        {
            *pData = ~*pData;
        }
        if(aPhase)
        {
            pSw  = pSw + aPhase;
            *pSw = ~*pSw;
        }
    }
}


UINT_8 is_pDataErased(const void *pData, UINT_32 len)
{
    if(len ==0 || len %4 != 0) // should be 4 bytes aligned
    {
        return 0;
    }
    UINT_32 *p_u32 = (UINT_32 *)pData;
    UINT_64 *p = (UINT_64 *)pData;
    if(len == 4)
    {
        return (*p_u32 == FLASH_ERASED_DATA_32BIT) ? 1 : 0;
    }
    else if(len == 8) // faster than "for loop"
    {
        return *p == FLASH_ERASED_DATA_64BIT ? 1 : 0;
    }
    else if(len == 16) // faster than "for loop"
    {
        return  (*p == FLASH_ERASED_DATA_64BIT) && (*(p + 1) == FLASH_ERASED_DATA_64BIT) ? 1 : 0;
    }
    else
    {
        UINT_8 isErased = 1;
        for (UINT_32 i = 0; i < len / 4; ++i)
        {
            if (*p_u32++ != FLASH_ERASED_DATA_32BIT)
            {
                isErased = 0;
                break;
            }
        }
        return isErased;
    }
}

void encodeTypeBinFile(unsigned int   encStrtAddr, unsigned int   encLen, char   encType)
{
    UINT_8   *pSw  = (UINT_8   *)&imageAddr[encStrtAddr - FLASH_STRTADDR]; // imageAddr store data with offset=FLASH_STRTADDR
    UINT_32  *p32  = (UINT_32  *)&imageAddr[encStrtAddr - FLASH_STRTADDR]; // imageAddr store data with offset=FLASH_STRTADDR
    UINT_64  *p64  = (UINT_64  *)&imageAddr[encStrtAddr - FLASH_STRTADDR]; // imageAddr store data with offset=FLASH_STRTADDR
    UINT_128 *p128 = (UINT_128 *)&imageAddr[encStrtAddr - FLASH_STRTADDR]; // imageAddr store data with offset=FLASH_STRTADDR
    UINT_32 addr;
    UINT_16 aPhase;
    switch (encType)
    {
        case ENCRYPT_POORMAN_U32:
        {
            fprintf(stderr, "ENCRYPT_POORMAN_U32\n");
            for (addr=encStrtAddr; addr<encStrtAddr + encLen; addr+=4) // 4 bytes one time
            {
                if(*p32 == 0x0 || *p32 == FLASH_ERASED_DATA_32BIT)
                {
                    p32 ++;
                    pSw += 4;
                    continue;
                }
                aPhase = (addr & 0x0c) >> 2;  //aPhase is 0-3
                if((aPhase&1)==0)
                {
                    *p32 = ~*p32;
                }
                if(aPhase)
                {
                    *(pSw + aPhase) = ~*(pSw + aPhase);
                }
                p32 ++;
                pSw += 4;
            }
            break;
        }
        case ENCRYPT_POORMAN_U128:
        {
            fprintf(stderr, "ENCRYPT_POORMAN_U128\n");
            for (addr=encStrtAddr; addr<encStrtAddr + encLen; addr+=16) // 16 bytes one time
            {
                if(  (p128->U64_Lo == 0x0 &&  p128->U64_Hi == 0x0)
                    || (p128->U64_Lo == FLASH_ERASED_DATA_64BIT &&  p128->U64_Hi == FLASH_ERASED_DATA_64BIT))
                {
                    p128 ++;
                    pSw += 16;
                    continue;
                }
                aPhase = (addr & 0x30) >> 4;  //aPhase is 0-3
                if((aPhase&1)==0) // aPhase bit0 ==0: invert all bytes
                {
                    p128->U64_Lo = ~p128->U64_Lo;
                    p128->U64_Hi = ~p128->U64_Hi;
                }
                if(aPhase) // aPhase bit1 ==1: encrpt first byte
                {
                    *(pSw + aPhase) = ~*(pSw + aPhase);
                }
                p128 ++;
                pSw += 16;
            }
            break;
        }
    
        default:
            break;
    }
    return;
}

// -----------------------------------------------------------------------------
// DSP IMAGE BIN FILE READ, WITHOUT FLASH ADDR OFFSET
// -----------------------------------------------------------------------------
unsigned int binFileRead(FILE *ifp)
{
    int  rc;
    int  bytes;
    //char buf;
    int  offs;
    int  blk;

    offs  = 0;
    bytes = 0;
    blk   = 0;
    while (1)
    {
        rc = fread( &imageAddr[offs], 1, 256, ifp );
        if ((rc==0) || (rc==EOF)) return bytes;
        offs  += rc;
        bytes += rc;
        blk++;
    }
    return 0;
}

// -----------------------------------------------------------------------------
// RELEASE BIN FILE WRITE, WITHOUT FLASH ADDR OFFSET
// -----------------------------------------------------------------------------
int binFileWrite(FILE *ofp, int offs, int numBytes)
{
    int  rc;
    //int  bytes;
    int  wb;
    int  blk;

    blk   = 0;
    while (1)
    {
        if (numBytes>=256) wb = 256;
        else               wb = numBytes;

        rc = fwrite( &imageAddr[offs], 1, wb, ofp );
        if (rc!=wb)
        {
            return -1;
        }
        // fprintf(stderr, "%d %d %d\n", blk, offs, rc );
        blk++;
        offs     += rc;
        numBytes -= 256;
        if (numBytes<=0) 
        {
            // fprintf(stderr, "Done %d\n", numBytes );
            break;
        }
    }
    return offs;
}


// -----------------------------------------------------------------------------
//
// -----------------------------------------------------------------------------
int  calcRealImgSize( int fType )
{
    int  i;
    //int  lastblk;
    int  endAddr;
    int  qwErase;
    int  qwErCnt;
    int  tQw;
    int  nQw;
    int  first = 1;

    qwErCnt = 0;
    if (fType == FILETYPE_IMG_A || fType == FILETYPE_IMG_B) // Img A/B
    {
        endAddr = imgStopAddr - 0x60 - 8; //last U64 except last 0x60 bytes
		tQw = imgStopAddr-imgStrtAddr;
    }
    else
    {
        return imgStopAddr-fleStrtAddr;
    }
    
    while ((endAddr>=imgStrtAddr) && (first))
    {
        qwErase = 1;
        for (i=0;i<8;++i) 
        {
            if (imageAddr[endAddr+i - FLASH_STRTADDR] != 0xff) // xqshao, imageAddr store data with offset=FLASH_STRTADDR
            {
                qwErase = 0;
                if (first)
                {
                    first = 0;
                    // fprintf(stderr, "first = %#08x  %d\n", endAddr, qwErCnt );
                }
                break;
            }
        }
        if (qwErase)
        {
            qwErCnt++;
        }
        endAddr -= 8;
    }
    tQw = (tQw - 0x60) / 8;  
    nQw = tQw - qwErCnt;
    // fprintf(stderr, "tQw %d - nQw %d - erase %d\n", tQw*8, nQw*8, qwErCnt*8 );
    // fprintf(stderr,"realImgSize %d\n", nQw*8 );
    return nQw*8;
}

// -----------------------------------------------------------------------------
// not used
// -----------------------------------------------------------------------------
int  genHdr( fileHdr_t  *hdr,
             char       *encStr,
             char       *fTypeStr,
             int        len )
{
	int binSize = 0;
	int xsize = 0;

    memcpy( hdr->lite, "LITE", 4 );
    
    // hdr->prodCode = 0xd4; // OSFP 1.1 and below
    hdr->prodCode = 0xd8;    // QSFPDD OSFP 800G 1.2 and above (until later)
    hdr->fileType = 0;
    if (strcmp(fTypeStr,"IMG-A")==0)  { hdr->fileType = 0x08; imgStrtAddr=0x10000; fleStrtAddr=0x0e000; xsize=8192; imgStopAddr=0x78000; binSize = 0x78000; }
    if (strcmp(fTypeStr,"IMG-B")==0)  { hdr->fileType = 0x88; imgStrtAddr=0x90000; fleStrtAddr=0x8e000; xsize=8192; imgStopAddr=0xf8000; binSize = 0x78000; }
    if (strcmp(fTypeStr,"DSP-N")==0)  { hdr->fileType = 0xD5; imgStrtAddr=0x00000; fleStrtAddr=0x00000; xsize=2048; imgStopAddr=0x00000; binSize = 0x00000; return 0; }

    hdr->prodCode = 0x50;    // STROSA 1.2 and above. (until later)
    if (strcmp(fTypeStr,"IMG-A")==0)  { hdr->fileType = 0x08; imgStrtAddr=0x10000; fleStrtAddr=0x0e000; xsize=8192; imgStopAddr=0x58000; binSize = 0x58000; }
    if (strcmp(fTypeStr,"IMG-B")==0)  { hdr->fileType = 0x88; imgStrtAddr=0x90000; fleStrtAddr=0x8e000; xsize=8192; imgStopAddr=0xd8000; binSize = 0x58000; }


    imgLen = imgStopAddr - imgStrtAddr;
    hdr->datalen = calcRealImgSize( hdr->fileType );
    hdr->totlen  = imgLen - 0x60; // full CRC excludes last 16 bytes
	return (binSize - 0x10000 + xsize);
}


// -----------------------------------------------------------------------------
//
// -----------------------------------------------------------------------------
void calcHdrChksum( fileHdr_t  *hdr )
{
    unsigned char  chksum;
    unsigned char *pDat;
    unsigned short i;

    chksum = 0;
    hdr->hdrChksum = 0;
    pDat = (unsigned char *) hdr;
    for (i=0;i<128;++i)
    {
        chksum += pDat[i];
    }
    hdr->hdrChksum = ~chksum;
}


// -----------------------------------------------------------------------------
//
// ARG
// 0     1       2       3        4    5    6
// imgAr outFile encType fileType date time inputfile
//               |       
//               ^       
//               enc0 ......NON-ENCRYPTED    
//               enc1 ......POOR MAN'S ENCRYPTED   
//                       |
//                       |
//                       ^
//                       IMG-A
//                       IMG-B
//                       DSP-N-A           dsp_vE000F200_ig1_A.bin
//                       DSP-N-B           dsp_vE000F200_ig1_B.bin
//                       NVR-REG           nvr_p254_b000_r128_l8192_v00000001_ig1.bin  
// -------------------------------------------&argv[6][4]->major.minor.build
//
// -----------------------------------------------------------------------------
int main( int argc, char *argv[] )
{
  FILE   *ifp;      // Pointer to input  
  FILE   *ofp;      // Pointer to output
  int     i;
  //int     chksumLow, chksumHigh;
  int     major, minor, build;
  unsigned int   ver_u32;
  int     flag_skipDspHdrImgA;
  int     nvr_page, nvr_bank, nvr_reg, nvr_len, nvr_ignoreMskRule, nvr_altBase;

  //unsigned char  byteChksum;  
  //unsigned int   crc32;
  //unsigned int   addr;

  //int     numBytes;
  int     fileSize;
  int     len;
  int     binSize;
//   int     isDsp;
  char    input_file[64];
  char    encStr  [8];
  char    fTypeStr[8];
  char    verStr[10];


  fileTrl_t  *qddTrl;



  strcpy_s( toolname, argv[0] );
  if (argc!=7) usage();
  // fprintf(stderr,"ArgC = %d\n", argc );

  memset( imageAddr,  0xff, 65536*16*4 ); //padding as 0xff by default
  memset( encStr,    0, 8 );
  memset( fTypeStr,  0, 8 );
  memset( verStr,    0, 10);


  // Open the file.
  if (strcmp(argv[1], "stdout") == 0)
  {
	  // fprintf(stderr, "Output to stdout\n");
	  ofp = stdout;
  }
  else
  {
	  // fprintf(stderr, "Output to %s\n", argv[1]);
	  fopen_s(&ofp, argv[1], "ab");
      // fprintf( stderr, "SeekEnd %d\n", fseek( ofp, 0, SEEK_END ) );
  }

  sscanf_s( argv[2], "%8c", encStr,   _countof(encStr)   );
  sscanf_s( argv[3], "%8c", fTypeStr, _countof(fTypeStr) );

  
  // -----------------------------------------------------------
  // Open and read the file into a buffer
  fprintf(stderr,"\n\nFTYPE = %s\n",fTypeStr );
  if (strcmp(fTypeStr,"DSP-N-A")==0 || strcmp(fTypeStr,"DSP-N-B")==0)
  {
      if (fopen_s( &ifp, argv[6], "rb" )!=0)
      {
          fprintf(stderr, "### Err opening input file: %s\n", argv[6] );
          exit(1);
      }
      fileSize = binFileRead(ifp);
      memcpy(verStr, &argv[6][5], 8); //"dsp_vE000F200_ig1_A.bin" -> E000F200
      char    verStr_u32[] = "0x00000000";
      memcpy(verStr_u32+2, verStr, 8);
      sscanf_s(verStr_u32, "%x", &ver_u32);
      major = (ver_u32>>24) & 0xff;
      minor = (ver_u32>>16) & 0xff;
      build = (ver_u32>> 0) & 0xffff;
      char    IgRuleStr[10];
      memcpy(IgRuleStr, &argv[6][16], 1); //"dsp_vE000F200_ig1_A.bin" -> 1
      IgRuleStr[1] = '\0';
      sscanf_s(IgRuleStr, "%d", &flag_skipDspHdrImgA);
  }
  else if(strcmp(fTypeStr,"NVR-REG")==0)
  {
      if (fopen_s( &ifp, argv[6], "rb" )!=0)
      {
          fprintf(stderr, "### Err opening input file: %s\n", argv[6] );
          exit(1);
      }
      fileSize = binFileRead(ifp);
    //   fprintf( stderr, "BinFile %s bytes = %d\n", argv[6], fileSize );
      char    nvrPageStr[10];
      char    nvrBankStr[10];
      char    nvrRegStr[10];
      char    nvrLenStr[10];
      char    nvrIgRuleStr[10];
      char    nvrAltBaseStr[10];
      // "nvr_p254_b000_r128_l8192_v00000001_ig1_alt1.bin"
      //       |    |    |    |     |          |    |

      //       ^    ^    ^    ^     ^          ^    ^
      //       5,3  10,3 15,3 20,4  26,8       37,1 42,1   ofst, len for each item.
      memcpy(nvrPageStr, &argv[6][5], 3); //"nvr_p254_b000_r128_l8192_v00000001_ig1_alt1.bin" -> 254
      nvrPageStr[3] = '\0';
      memcpy(nvrBankStr, &argv[6][10], 3); //"nvr_p254_b000_r128_l8192_v00000001_ig1_alt1.bin" -> 000
      nvrBankStr[3] = '\0';
      memcpy(nvrRegStr, &argv[6][15], 3); //"nvr_p254_b000_r128_l8192_v00000001_ig1_alt1.bin" -> 128
      nvrRegStr[3] = '\0';
      memcpy(nvrLenStr, &argv[6][20], 4); //"nvr_p254_b000_r128_l8192_v00000001_ig1_alt1.bin" -> 8192
      nvrLenStr[4] = '\0';
      memcpy(verStr, &argv[6][26], 8); //"nvr_p254_b000_r128_l8192_v00000001_ig1_alt1.bin" -> 00000001
      verStr[8] = '\0';
      memcpy(nvrIgRuleStr, &argv[6][37], 1); //"nvr_p254_b000_r128_l8192_v00000001_ig1_alt1.bin" -> 1
      nvrIgRuleStr[1] = '\0';
      memcpy(nvrAltBaseStr, &argv[6][42], 1); //"nvr_p254_b000_r128_l8192_v00000001_ig1_alt1.bin" -> 1
      nvrAltBaseStr[1] = '\0';
      //   fprintf( stderr, "verStr %s, page %s, bank %s, ignoreMskRule %s\n", verStr, nvrPageStr, nvrBankStr, nvrIgRuleStr );
      char    verStr_u32[] = "0x00000000";     
      memcpy(verStr_u32+2, verStr, 8);
      sscanf_s(verStr_u32, "%x", &ver_u32);
    //   fprintf(stderr, "ver_u32 = %#010x\n", ver_u32 );
      major = (ver_u32>>24) & 0xff;
      minor = (ver_u32>>16) & 0xff;
      build = (ver_u32>> 0) & 0xffff;  

      sscanf_s(nvrPageStr, "%d", &nvr_page);
      sscanf_s(nvrBankStr, "%d", &nvr_bank);
      sscanf_s(nvrRegStr, "%d", &nvr_reg);
      sscanf_s(nvrLenStr, "%d", &nvr_len);
      sscanf_s(nvrIgRuleStr, "%d", &nvr_ignoreMskRule);
      sscanf_s(nvrAltBaseStr, "%d", &nvr_altBase);
  }
  else if( strcmp(fTypeStr,"IMG-A")==0 || strcmp(fTypeStr,"IMG-B")==0 ) 
  {
      if (fopen_s( &ifp, argv[6], "r" )!=0)
      {
          fprintf(stderr, "### Err opening input file: %s\n", argv[6] );
          exit(1);
      }
      fileSize = hexReadData( ifp );

    //   fprintf( stderr, "HexFile %s lines = %d\n", argv[6], fileSize );
    //   fprintf( stderr, "\n\n");
  }
  else
  {
      fprintf(stderr, "### Err: unsupported file type %s\n", fTypeStr );
      exit(1);
  }
  fclose(ifp);
  strcpy_s( input_file, argv[6] );


  // NOTE: Header is 256 bytes for space for encryption keys.

// #if STROSA
//   binSize = genHdr( &qddHdr, encStr, fTypeStr, 0x58000-0x10000-0x10 );
// #else
//   binSize = genHdr( &qddHdr, encStr, fTypeStr, 0x78000-0x10000-0x10 );
// #endif
    
    // Header generation
    memset(&qddHdr, 0, sizeof(qddHdr) );
    memcpy( &qddHdr.lite, "LITE", 4 );
    
    qddHdr.prodCode = PRODCODE_1600_DR8_SIAN3;    // QSFPDD OSFP 800G 1.2 and above (until later)
    qddHdr.fileType = 0;
    if (strcmp(fTypeStr,"IMG-A")==0)  
    { 
        qddHdr.fileType = FILETYPE_IMG_A;     
        imgStrtAddr = APPL_IMGA_VECTOR; 
        fleStrtAddr = APPL_IMGA_FW_CFG_DATA; 
        imgStopAddr = APPL_IMGA_CRC32_END; 
        binSize = imgStopAddr-fleStrtAddr; 
        qddHdr.totlen  = imgStopAddr - imgStrtAddr - 0x60; // full CRC excludes last 0x60 bytes and first 0x2000 bytes
        qddHdr.datalen = calcRealImgSize( FILETYPE_IMG_A ); // real image size excluding padding 0xff from totlen
        qddHdr.imglen  = binSize; // all data from BOOT_CTRL_BLK_IMGB(0xe000) to fullImgCrc32(0xe78000) will be wrote to flash
        memcpy(verStr, &imageAddr[APPL_IMGA_FW_VER_STRING - FLASH_STRTADDR], 8);
        sscanf_s(verStr, "%d.%d.%d", &major, &minor, &build);
        // isDsp = 0;
        fprintf( stderr, "%s totlen = %#x, datalen= %#x, imglen = %#x, major = %d, minor = %d, build = %d\n", fTypeStr, qddHdr.totlen, qddHdr.datalen, qddHdr.imglen, major, minor, build );
        
    } 
    else if (strcmp(fTypeStr,"IMG-B")==0)  
    { 
        qddHdr.fileType = FILETYPE_IMG_B;     
        imgStrtAddr = APPL_IMGB_VECTOR; 
        fleStrtAddr = APPL_IMGB_FW_CFG_DATA; 
        imgStopAddr = APPL_IMGB_CRC32_END; 
        binSize = imgStopAddr-fleStrtAddr; 
        qddHdr.totlen  = imgStopAddr - imgStrtAddr - 0x60; // full CRC excludes last 0x60 bytes and first 0x2000 bytes
        qddHdr.datalen = calcRealImgSize( FILETYPE_IMG_B ); // real image size excluding padding 0xff from totlen
        qddHdr.imglen  = binSize; // all data from BOOT_CTRL_BLK_IMGB(0xe000) to fullImgCrc32(0xe78000) will be wrote to flash
        memcpy(verStr, &imageAddr[APPL_IMGB_FW_VER_STRING - FLASH_STRTADDR], 8); //defined in memmap 0x1c0
        sscanf_s(verStr, "%d.%d.%d", &major, &minor, &build);
        fprintf( stderr, "%s totlen = %#x, datalen= %#x, imglen = %#x, major = %d, minor = %d, build = %d\n", fTypeStr, qddHdr.totlen, qddHdr.datalen, qddHdr.imglen, major, minor, build );
            // isDsp = 0;
    }
    else if (strcmp(fTypeStr,"DSP-N-A")==0 || strcmp(fTypeStr,"DSP-N-B")==0)
    {   
        if (strcmp(fTypeStr,"DSP-N-A")==0)
        {
            qddHdr.fileType = FILETYPE_DSP_SIAN2_IMG_A;     
        }
        else
        {
            qddHdr.fileType = FILETYPE_DSP_SIAN2_IMG_B;
        }    
        if ((fileSize & 0xff)!=0)  //padded as N*256
        {
            fileSize = (fileSize>>8);
            fileSize =  fileSize + 1;
            fileSize = (fileSize<<8);
        }
        imgStrtAddr=0; 
        fleStrtAddr=0; 
        imgStopAddr=fileSize; 
        qddHdr.datalen = fileSize; //better to not use qddTrl in dsp binary
        qddHdr.totlen  = fileSize; //better to not use qddTrl in dsp binary
        qddHdr.imglen  = fileSize;
        qddHdr.flag_skipDspHdrImgA = flag_skipDspHdrImgA;
        binSize        = fileSize;
        fprintf( stderr, "%s totlen = %#x, datalen= %#x, imglen = %#x,  ver_u32 = %#010x, major = %d, minor = %d, build = %d, flag_skipDspHdrImgA = %d\n", fTypeStr, qddHdr.totlen, qddHdr.datalen, qddHdr.imglen, ver_u32, major, minor, build, flag_skipDspHdrImgA );

    }
    else if(strcmp(fTypeStr,"NVR-REG")==0)
    {
        qddHdr.fileType = FILETYPE_NVR_REG;     
        imgStrtAddr = 0; 
        fleStrtAddr = 0; 
        imgStopAddr = fileSize; 
        qddHdr.datalen = nvr_len; //this is real nvr data len, might not be n*256
        qddHdr.totlen  = nvr_len; 
        qddHdr.imglen  = nvr_len;
        binSize        = fileSize; //need to write all data(with padding) in bin file as n*256 blocks.
        qddHdr.nvr_page = nvr_page;
        qddHdr.nvr_bank = nvr_bank;
        qddHdr.nvr_reg_addr = nvr_reg;
        qddHdr.ignoreNvrMskRule = nvr_ignoreMskRule;
        qddHdr.flag_nvrAltBase = nvr_altBase;
        fprintf( stderr, "%s totlen = %#x, datalen= %#x, imglen = %#x,  ver_u32 = %#010x, major = %d, minor = %d, build = %d, page = %d, bank = %d, reg = %d, ignoreMskRule = %d, altBase = %d\n", fTypeStr, qddHdr.totlen, qddHdr.datalen, qddHdr.imglen, ver_u32, major, minor, build, qddHdr.nvr_page, qddHdr.nvr_bank, qddHdr.nvr_reg_addr, nvr_ignoreMskRule, nvr_altBase );
    }

    // crc calculation and check
    if( strcmp(fTypeStr,"IMG-A")==0 || strcmp(fTypeStr,"IMG-B")==0 ) //crc stored both in trailer and header for image A/B
    {
        unsigned int   crc32_fullImg, crc32_realImg;
        // Overwrite last xx bytes...
        // Calc the CRC of data excluding erased image... 
        qddTrl = (fileTrl_t *) &imageAddr[imgStopAddr-16 - FLASH_STRTADDR] ; //xqshao, mcu fw image, stored in imageAddr with offset=FLASH_STRTADDR

        crc32_realImg  = calcCrc32( &imageAddr[imgStrtAddr - FLASH_STRTADDR], qddHdr.datalen, 0 );
        qddTrl->imgCrc32 = crc32_realImg;
        // fprintf(stderr,"strt=%#08x realImgCrc32=%#010x dlen=%d\n", imgStrtAddr, crc32, qddHdr.datalen );

        crc32_fullImg   = calcCrc32( &imageAddr[imgStrtAddr - FLASH_STRTADDR], qddHdr.totlen , 0 );
        // fprintf(stderr,"strt=%#08x fullImgCrc32=%#010x dlen=%d\n", imgStrtAddr, crc32, qddHdr.totlen  );
        fprintf(stderr,"%s strt=%#08x fullImgCrc32=%#010x totlen=%#x realImgCrc32=%#010x datalen=%#x\n", fTypeStr, imgStrtAddr, crc32_fullImg, qddHdr.totlen, crc32_realImg, qddHdr.datalen );
        if (crc32_fullImg != qddTrl->fullImgCrc)
        {
            fprintf(stderr,"CRC mismatch %#010x :: %#010x\n", crc32_fullImg, qddTrl->fullImgCrc );
        }
        else
        {
            fprintf(stderr,"CRC MATCH    %#010x :: %#010x\n", crc32_fullImg, qddTrl->fullImgCrc );
        }
        qddHdr.imgCrc32 = crc32_fullImg;
    }
    else if (strcmp(fTypeStr,"DSP-N-A")==0 || strcmp(fTypeStr,"DSP-N-B")==0) //crc stored only in header, better to not use qddTrl in dsp binary for we cann't modify last 16 bytes of it
    {
        // qddTrl = (fileTrl_t *) &imageAddr[imgStopAddr-16] ;
        qddHdr.imgCrc32 = calcCrc32( &imageAddr[imgStrtAddr], qddHdr.totlen, 0 ); //xqshao, dsp image buff, without address offset
        // fprintf(stderr,"strt=%#08x crc32=%#010x dlen=%d\n", imgStrtAddr, qddHdr.imgCrc32, qddHdr.totlen );
        int dspImgB_strtAddr = imgStrtAddr + DSP_IMGB_STRTADDR_OFST;
        int dspImgB_len = qddHdr.totlen - DSP_IMGB_STRTADDR_OFST;
        qddHdr.imgCrc32_dspB = calcCrc32( &imageAddr[dspImgB_strtAddr], dspImgB_len, 0 );
        // fprintf(stderr,"strt=%#08x crc32_dspB=%#010x dlen=%d\n", dspImgB_strtAddr, qddHdr.imgCrc32_dspB, dspImgB_len );
         fprintf(stderr,"%s strt=%#08x crc32=%#010x totlen=%#x strt=%#08x crc32_dspB=%#010x datalen=%#x\n", fTypeStr, imgStrtAddr, qddHdr.imgCrc32, qddHdr.totlen, dspImgB_strtAddr, qddHdr.imgCrc32_dspB, dspImgB_len );
    }
    else if(strcmp(fTypeStr,"NVR-REG")==0) //crc stored only in header
    {
        qddHdr.imgCrc32 = calcCrc32( &imageAddr[imgStrtAddr], qddHdr.totlen, 0 ); 
        fprintf(stderr,"%s strt=%#08x crc32=%#010x dlen=%#x\n", fTypeStr, imgStrtAddr, qddHdr.imgCrc32, qddHdr.totlen );
    }

//   fprintf(stderr, "info -> %s %s %d.%d.%d   %d\n", encStr, fTypeStr, major, minor, build, binSize);

  qddHdr.major = major;
  qddHdr.minor = minor;
  qddHdr.build = build;
  len = strlen(input_file);
  if (len>=32)
  {
      memcpy( qddHdr.filename, input_file, 32 );
  }
  else
  {
      memcpy( qddHdr.filename, input_file, len );
      memset( &qddHdr.filename[len], 0, 32-len );
  }
  memcpy( qddHdr.date, argv[4], 8 );
  memcpy( qddHdr.time, argv[5], 8 );
  memset( qddHdr.resv,    0,  sizeof(qddHdr.resv));
  memset( qddHdr.encKey,  0, sizeof(qddHdr.encKey) );

  // Need to add 16 bytes at the end of the file for checksum and other flags
  // numBytes = fileSize + 16;
  // if (numBytes%256)
  // {
      // numBytes = ((numBytes/256)+1) * 256;
      // fprintf(stderr,"Padded to 256 bytes : %d", numBytes );
  // }

#if 1
  // Encode the file
//   fprintf(stderr," (%s)", encStr );
  qddHdr.encType = ENCRYPT_NONE;
  if (strcmp(encStr,"enc1")==0) //poor man's encryption
  {
    if (strcmp(fTypeStr,"IMG-A")==0 || strcmp(fTypeStr,"IMG-B")==0 )  // no need to encrpt for dsp binary and nvr reg binary
    { 
        qddHdr.encStrtAddr = imgStrtAddr + 0x250; //except VIC
        qddHdr.encLen      = qddHdr.datalen - 0x250; //except VIC
        qddHdr.encType = ENCRYPT_POORMAN_U128;
        // encodeType1BinFile(1, fileSize);
        encodeTypeBinFile(qddHdr.encStrtAddr, qddHdr.encLen, ENCRYPT_POORMAN_U128);
        fprintf(stderr, "%s ### Encoded Type1(PoorMan): %#x, encStrtAddr: %#010x, encLen: %#x\n", fTypeStr, qddHdr.encType & 0xf, qddHdr.encStrtAddr, qddHdr.encLen );
    }
    
  }
//   fprintf(stderr, "\n");
#endif

  // Write Header
  fprintf(stderr,"calc-hdr chk\n");
  calcHdrChksum( &qddHdr );
  fwrite( &qddHdr, 1, 256, ofp );

  // Write Data 
  
//   fprintf(stderr,"Strt = %#010x %#x\n", fleStrtAddr, binSize );
  int binFileOfst = fleStrtAddr;
  if (strcmp(fTypeStr,"IMG-A")==0 || strcmp(fTypeStr,"IMG-B")==0 )
  {
        binFileOfst = fleStrtAddr - FLASH_STRTADDR; // IMG-A/B fleStrtAddr need to minus FLASH_STRTADDR
  }

  i = binFileWrite( ofp, binFileOfst, binSize );
  // if (i!=1024)
  {
    //   fprintf(stderr,"Wrote = %#x\n", i );
      fprintf(stderr,"write to ReleaseBin: buffOfst = %#010x lens=%#x\n", binFileOfst, binSize );
      fclose(ofp);
      exit(1);
  }

  return 0;
}
