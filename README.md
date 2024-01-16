# Rust_C_Imports_Derive
This is a crate to make it easier to sync values in header files between your rust code and other C/C++ code. It solves the problem of having duplicate values in your rust and C/C++ code since rust can not use values in header files.
With this crate you can have all your values in header files and use proc macros to include them in your rust code before compiling.

## Usage
This crate provides one proc macro from_c_header!.

my_header.h
```c
#define MY_VALUE 7
#define MY_OTHER_VALUE 20
#define MY_STRING_VALUE "this is a string"

#define MY_OTHER_VALUE_COPIED MY_OTHER_VALUE

#define LARGE_VALUE 5000
```
Values in my_header.h can be used using the macro:
```rust
use rust_c_imports_derive::from_c_header;

from_c_header! {
  const MY_VALUE: u8 in "./my_header.h";
  const MY_OTHER_VALUE: usize in "./my_header.h";
  const MY_STRING_VALUE: &str in "./my_header.h";

  const MY_OTHER_VALUE_COPIED: usize in "./my_header.h";
}
```
will be converted to:
```rust
const MY_VALUE: u8 = 7;
const MY_OTHER_VALUE: usize = 20;
const MY_STRING_VALUE: &str = "this is a string";

MY_OTHER_VALUE_COPIED: usize = MY_OTHER_VALUE;
```
**Note**: MY_OTHER_VALUE_COPIED needs MY_OTHER_VALUE to be included in order to compile.

If the value is not compatible with the type you provided it will not compile.
```rust
  const LARGE_VALUE: u8 in "./my_header.h";
```
becomes
```rust
const LARGE_VALUE: u8 = 5000;
```
which does not compile because 5000 does not fit in u8.

**Note**: Paths are based on root of the crate the file is in not the source file itself.

**Note**: If the header file changes, the rust file will be recompiled.

## Limitations:
This crate does not currently parse the header file to resolve what the value will be when compiled. This is a much more complicated problem that this crate does not implement. It only finds the define statment and copy paste the text after the define.
This means if the value after the define is not valid rust it will not compile.

