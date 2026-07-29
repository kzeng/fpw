#include "stdafx.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

const char *hexFileEof = ":00000001FF";

// ======================================================================================
static void printHexChar( char *str, int hexChar )
{
  

  if (hexChar<10)   sprintf_s( str, 3, "%c", '0'+hexChar );
  else              sprintf_s( str, 3, "%c", 'A'+(hexChar-10) );
}

// ======================================================================================
void hexPrintSize( char *str, int size )
{
  if (size>=16)   sprintf_s( str, 4, "10");
  else            
  {
    sprintf_s( str, 4, "0" );
    printHexChar( str+1, size );
  }
}


// ======================================================================================
void hexPrintAddr( char *str, unsigned short addr )
{
  int outAddr;

  outAddr = (addr & 0xF000) >> 12;
  printHexChar( str,  outAddr);
  outAddr = (addr & 0x0F00) >> 8;
  printHexChar( str+1,  outAddr);
  outAddr = (addr & 0x00F0) >> 4;
  printHexChar( str+2,  outAddr);
  outAddr = (addr & 0x000F) >> 0;
  printHexChar( str+3,  outAddr);
}


// ======================================================================================
void hexPrintChar( char *str, unsigned char data )
{
  int outData;

  outData = (data & 0xF0) >> 4;
  printHexChar( str  , outData);
  outData = (data & 0x0F) >> 0;
  printHexChar( str+1, outData);

}

void extendedAddressLine( FILE *ofp, unsigned int addr)
{
  char   strbuf[48];
  char  *str;
  unsigned int  chksum;

  str = &strbuf[0];
  sprintf_s( str, 4, ":");      str+=1;
  hexPrintSize( str, 2);   str+=2;
  hexPrintAddr( str, 0 );  str+=4;
  sprintf_s( str, 4, "04");     str+=2;
  sprintf_s( str, 4, "00");     str+=2;

  addr = (addr >> 16) & 0xff;
  hexPrintChar( str, addr);  str+=2;
  chksum = 2 + 4 + addr;
  chksum &= 0xFF;
  chksum  = 0x100 - chksum;
  chksum &= 0xFF;
  hexPrintChar( str, chksum );  str+=2;
  *str = 0;
  fprintf( ofp, "%s\n", strbuf );
}

// ======================================================================================
void printIntelHexData( FILE *ofp, int extAddr, int addrStart, unsigned char *buf, int size )
{
  unsigned int  outAddr;
  int    i,j;
  int    dsize;
  char   strbuf[48];
  char   xbuf[48];
  char  *str;
  unsigned int  chksum;
  unsigned char *tstBuf;

  /*
  if (extAddr>=0)
  {
    str = &strbuf[0];
    sprintf_s( str, 4, ":");      str+=1;
    hexPrintSize( str, 2);   str+=2;
    hexPrintAddr( str, 0 );  str+=4; 
    sprintf_s( str, 4, "04");     str+=2;
    sprintf_s( str, 4, "00");     str+=2;
    hexPrintChar( str, extAddr&0xff );  str+=2;
    chksum = 2 + 4 + (extAddr&0xff);
    chksum &= 0xFF;
    chksum  = 0x100 - chksum;
    chksum &= 0xFF;
    hexPrintChar( str, chksum );  str+=2;
    *str = 0;
    fprintf( ofp, "%s\n", strbuf );
  }*/
  extendedAddressLine( ofp, extAddr);

  outAddr = addrStart & 0xFFFF;
  tstBuf = buf;
  do
  {
    int isErased1, isErased2;
	

    str = &strbuf[0];
    if      (size>16)    { dsize = 16  ; size -= 16; }
    else if (size == 0)  { break; }
    else                 { dsize = size; size = 0; }

	// Logic only works if ranges are multiple of 8 and 16
	isErased1 = 1;
	for (i = 0; i < 8; ++i)
	{
		if (*tstBuf != (unsigned char)0xFF) isErased1 = 0;
		tstBuf++;
	}
	if (dsize > 8)
	{
	    isErased2 = 1;
		for (i = 8; i < dsize; ++i)
		{
			if (*tstBuf != (unsigned char)0xFF) isErased2 = 0;
			tstBuf++;
		}
	}


	if ((isErased1 != 0) && (isErased2 != 0))
	{
		buf += 16;
	}
	else
	{
		if ((isErased1 == 0) && (isErased2 == 0))
		{
		     // do nothing.
		}
		if ((isErased1 == 0) && (isErased2 != 0))
		{
			dsize = 8;
		}
		if ((isErased1 != 0) && (isErased2 == 0))
		{
			dsize = 8; outAddr += 8;
			buf += 8;
		}

		sprintf_s(str, 4, ":");      str += 1;
		hexPrintSize(str, dsize);    str += 2;
		hexPrintAddr(str, outAddr);  str += 4;

		chksum = dsize;
		chksum += (outAddr & 0xFF00) >> 8;   chksum += outAddr & 0xFF;

		// Hex Type Id = Data
		sprintf_s(str, 4, "00");        str += 2;

		// Data Size print char and checksum.
		for (i = 0; i<dsize; ++i)
		{
			hexPrintChar(str, *buf);  str += 2;
			chksum += *buf;
			buf++;
		}

		if ((isErased1 == 0) && (isErased2 != 0))
		{
			buf += 8;
		}

		chksum &= 0xFF;
		chksum = 0x100 - chksum;
		chksum &= 0xFF;
		hexPrintChar(str, chksum);  str += 2;
		*str = 0;

		fprintf(ofp, "%s\n", strbuf);
	}

  



    
    outAddr+=16;
	if (outAddr >= 0x10000)
	{
		extAddr = extAddr + 0x10000;
		extendedAddressLine( ofp, extAddr);
		outAddr = outAddr - 0x10000;
	}

  } while (size>0);
}



// -----------------------------------------------------------------------------
//
// -----------------------------------------------------------------------------
unsigned char hexToByte( char *str )
{
  char  chexStr[50];
  unsigned int val;
  

  sprintf_s( chexStr, 50, "0x", 2 );

  //sprintf_s( &chexStr[2], 48, "0x", 2 );
  chexStr[2] = str[0];
  chexStr[3] = str[1];
  // strncpy_s( &chexStr[2], 2, str, 2);
  
  chexStr[4] = 0;
  
  sscanf_s( chexStr, "%x", &val, 1 );
  return val;
}



// -----------------------------------------------------------------------------
// 
// -----------------------------------------------------------------------------
unsigned char calcHexLineChecksum( char *hexLine  )
{
  int  start;
  int  size;
  int  i;
  unsigned int  chksum;

  chksum = 0;
  size = hexToByte( &hexLine[1] );
  for ( start=1, i=0 ;  i<(4+size) ; start+=2, ++i )
  {
    chksum += hexToByte( &hexLine[start] );
  }
  chksum &= 0xFF;
  chksum  = 0x100 - chksum;
  chksum &= 0xFF;
  return chksum;
}

// -----------------------------------------------------------------------------
// 
// -----------------------------------------------------------------------------
int hexGetAddr( char *hexline )
{
  unsigned char  lobyte, hibyte;

  if ( hexline[0] != ':') return 0xf0000;

  hibyte = hexToByte( &hexline[3] );
  lobyte = hexToByte( &hexline[5] );

  return( (hibyte<<8) | (lobyte) );
}

// -----------------------------------------------------------------------------
// 
// -----------------------------------------------------------------------------
void hexLineReplaceChksum( char *hexline, unsigned char chksum )
{
  int  size;
  int  offset;

  size   = hexToByte( &hexline[1] );
  offset = 9 + size*2;
  hexPrintChar( &hexline[offset], chksum );
}

