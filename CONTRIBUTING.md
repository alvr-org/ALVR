# Style

Checklist for code style. This is on top of common Rust styling rules. These rules are not mandatory but I might point them out if not respected in PRs :) -zarik

## Naming

- Respect Rust naming conventions (not respecting this will cause a warning).
- Add useful information in the name, with the exception of indices for iterating.
- Do not put type or scope information in the name.
- Avoid abbreviations.
- Avoid prefixes and suffixes.
- if necessary prefer suffixes rather than prefixes.
- `_ref` or `_mut` suffixes are accepted when you want to put emphasis on that the variable is a mutable reference, and so assigning values has side effects, even if not consumed later. Suffixes are not needed if the variable is only mutable or instead a immutable reference.
- Use `maybe_` prefix if the variable is an `Option` or `Result`. Omit if it's clear from the context. Never use it for parameter or field definitions.
- Shadowing is encouraged.
- If shadowing cannot be used and two variables have similar meaning but different types, suffix the variable with the least useful or least specialized type with its type. Example:

    ```rust
    let myfile_string = "./file.txt";
    let myfile = Path::new(my_file_string);
    ```

- If both directory and file paths are used in the same context, suffix directories with `_dir` and files with `_path`. Suffix file names with `_fname`.

## Top level definitions

- For each file, define in order: private imports, public imports, ffi bindings import, private constants, public constants, private structs, public structs, private top level functions, public top level functions.

### Imports

- Do not leave spaces between imports, only between the private and public import blocks.
- Define imports in alphabetical order (use cargo fmt).
- Group imports using braces when there are common parts of the path.

### Structs

- Prefer adding trait bounds to the impl type parameters instead of struct type parameters
- Define in order: struct, Default impl, custom impl, Drop impl, all in the same module. Do not split the custom impl.

## Spacing

Smartly use empty newlines between blocks of code to improve legibility

- Rust recommends not to specify the return keyword when returning at the end of a function. To put emphasis on the return expression, isolate it with a empty new line just before.
- Inside each block (function, if/else/while/for etc or just braces) make so that there are roughly 2 to 6 blocks of code separated by empty lines for the amount of code that fits in a single screen (long functions that span multiple screens can have way more than 6 block). Check existing codebase for an example.
- Spaces between groups of statements should be done so the groups are similar is size and that each achieves a specific purpose. You should be able to easily describe what the group does in few words, even if you don't comment it (because the meaning should be self evident).
- Do not define variables at the start of the function/block, but do define them at the start of the functional group delimited by spaces.

## Comments

- It's important to use comments when the meaning or inner workings of a piece of code is not clear from the context. Well-named symbols (variables and functions) are often enough. In doubt do use comments.
- Do not add comments about how certain parts of the language/std library work, unless it's about quirks of features.

## Panicking and error handling

- Use of `panic!()` is discouraged
- When matching exhaustively, prefer `unreachable!()` over `panic!()` for certain branches.
- `unwrap()` is discouraged, but prefer `unwrap()` over `expect()` (bubble up the error instead).
- Prefer `.get()` to index a collection rather than `[]`, unless extremely certain it will never index out of bounds.
- Add a `// # Safety` comment before a statement that contains a `unwrap()` or raw indexing, explaining why it should never crash.
- Use `todo!()` to mark unfinished code (it returns `!` and so it helps with the missing return statement).

## Code repetition and maintainability

- Lean towards the DRY rule, without overdoing it.
- Extract a piece of code (in a function or lambda) only when it is used two or more times and it doesn't depend on many parameters.
- Always extract constants for "arbitrary" values (literals) which are chosen with no absolute rule and makes sense to change in the future. Example: time interval between resending discovery packet. Opposite example: number of eyes on a human head (it's always going to be 2, no need to use a constant to change its value in the future :) ).
- Prefer defining constants at the start of the file, even if used locally in a single function.
- Prefer using "complex" types for constants, if the std library allows it. Example: prefer using `Duration` instead of an integer type if the constant represents a time duration. Same with `Path` vs string.

## Structural soundness

- Try to avoid invalid states in the data, using Rust enums. Example: do not use `resumed` + `streaming` boolean variables if the state `resumed == false` + `streaming == true` is invalid. Instead use `enum State { Paused, Resumed, Streaming }`.
- Make full use of pattern matching with `if let` and `while let`, this reduces the use of `unwrap()`.
