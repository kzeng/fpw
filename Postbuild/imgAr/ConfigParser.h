#ifndef __CONFIG_PARSET_H_
#define __CONFIG_PARSET_H_



// Maximum I2C Pages.  This number really can't change. 
#define  MAX_PAGES           128

// In the configuration file, each page could have the following
// maximum number of lines.
#define  MAX_LINES_PER_PAGE  150


typedef struct
{
    int  chksum_byte_count;
    int  chksum_start;
    int  chksum_len;
    int  chksum_location;
} page_byte_chksum_t;

typedef struct
{
    char  segName[32];
    int   configured;
    int   page;
    int   extAddr;
    int   hexStartAddr;
    int   num16ByteLines;
    int   regAddrOffset;
    page_byte_chksum_t  chksum[8];
} hexSegHdr_t;


typedef struct
{
  char  *sfmt;
  char   dtsize;
  char   isSigned;
  char   bigEndian;
  char   isFloat;
} dataTypeConv_t;


typedef struct
{
    int  configured;
    int  regAddr;
    int  size;
    dataTypeConv_t  dtype;
    int  prtf_fmt1;
    int  prtf_fmt2;
    int  prtf_fmt2_precision;
    float  prtf_fmt2_scale;
    char fmt1String[128];
    char fmt2String[128];
} hexSegData_t;



int  parseConfigFile( FILE *fp,
                      hexSegHdr_t  *cfgHdr,
                      hexSegData_t *cfgData );

unsigned short convertFloat32_16( float f32 );

void  displayConfig( int            page,
                     hexSegHdr_t  *cfgHdr,
                     hexSegData_t *cfgData,
                     unsigned char *hexFileData, 
                     unsigned char *hexDiffData, 
                     int            extAddrBufOffset );
#endif
