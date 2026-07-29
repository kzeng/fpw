
#ifndef __HEX_FILE_LIBS_H
#define __HEX_FILE_LIBS_H

extern const char *hexFileEof;

void printSize( char *, int size );
void printAddr( char *, unsigned short addr );
void hexPrintChar( char *, unsigned char data );
void printIntelHexData( FILE *ofp, int extAddr, int addrStart, unsigned char *buf, int size );

unsigned char hexToByte( char *str );
unsigned char calcHexLineChecksum( char *hexLine  );
int hexGetAddr( char *hexline );
void hexLineReplaceChksum( char *hexlineN, unsigned char chksum );

#endif
