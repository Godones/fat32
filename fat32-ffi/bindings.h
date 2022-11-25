#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

enum TagType {
  File,
  Dir,
};
typedef uint8_t TagType;

typedef struct String String;

typedef struct Vec_Dir Vec_Dir;

typedef struct Vec_File Vec_File;

typedef struct Tag {
  struct String name;
  struct Vec_File files;
  struct Vec_Dir dirs;
} Tag;

void rust_function(struct Tag tag);

void rust_function2(TagType ctype);
