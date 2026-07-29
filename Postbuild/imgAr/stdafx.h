// stdafx.h : include file for standard system include files,
// or project specific include files that are used frequently, but
// are changed infrequently
//

#pragma once

#include "targetver.h"

#include <stdio.h>
#include <tchar.h>

typedef signed char         INT_8;
typedef unsigned char       UINT_8;
typedef signed short        INT_16;
typedef unsigned short      UINT_16;
typedef signed int          INT_32;
typedef unsigned int        UINT_32;
typedef signed long long    INT_64;
typedef unsigned long long  UINT_64;
typedef unsigned long long  BUINT_64;

typedef union {
    struct
    {
        UINT_64 U64_Lo;
        UINT_64 U64_Hi;
    };
    UINT_32 U32[4];
    UINT_16 U16[8];
    UINT_8  U8[16];
} UINT_128;

#define FLASH_ERASED_DATA_32BIT     0xFFFFFFFF
#define FLASH_ERASED_DATA_64BIT     0xFFFFFFFFFFFFFFFF  // used by EEPROM

// TODO: reference additional headers your program requires here
