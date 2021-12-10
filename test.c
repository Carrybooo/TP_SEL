#include <stdlib.h>
#include <malloc.h>
#include <stdio.h>
#include <errno.h>

#include <unistd.h>
#include <signal.h>
#include <sys/mman.h>

int main(){

  void* ptr;
  void* ptr2;
  void* ptr3;
  void* ptr4;
  void* ptr5;
  void* ptr6;
  void* ptr7;

  printf("ptr value before posix_memalign : %d %d %d %d %d %d %d\n",
   ptr, ptr2, ptr3, ptr4, ptr5, ptr6, ptr7);

  int a = posix_memalign(&ptr, 4096, 4096);

  printf("a : %d | et ptr : %d\n", a, ptr);
  printf("EINVAL:%d \nENOMEM:%d \nsizeof(sizet):%d\n",
  EINVAL, ENOMEM, sizeof(size_t));


  printf("prot read : %d, prot write : %d, prot exec : %d,\n",
   PROT_READ, PROT_WRITE, PROT_EXEC);
}

/*
00
01
10
11
*/
