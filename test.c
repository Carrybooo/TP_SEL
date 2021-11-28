#include <stdlib.h>
#include <malloc.h>
#include <stdio.h>
#include <errno.h>

int main(){

  void* ptr;

  int a = posix_memalign(&ptr, 4096, 4096);

  printf("a : %d | et ptr : %d\n", a, ptr);
  printf("return:%d %d %d\n",EINVAL, ENOMEM, sizeof(size_t));

}
