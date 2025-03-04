# The EndBASIC programming language - CLI

![Build and test](https://github.com/jmmv/endbasic/workflows/Build%20and%20test/badge.svg)
![Health checks](https://github.com/jmmv/endbasic/workflows/Health%20checks/badge.svg)
![Deploy to staging](https://github.com/jmmv/endbasic/workflows/Deploy%20to%20staging/badge.svg)
![Deploy to release](https://github.com/jmmv/endbasic/workflows/Deploy%20to%20release/badge.svg)
[![Crates.io](https://img.shields.io/crates/v/endbasic.svg)](https://crates.io/crates/endbasic/)
[![Docs.rs](https://docs.rs/endbasic/badge.svg)](https://docs.rs/endbasic/)

EndBASIC is an interpreter for a BASIC-like language and is inspired by
Amstrad's Locomotive BASIC 1.1 and Microsoft's QuickBASIC 4.5.  Like the former,
EndBASIC intends to provide an interactive environment that seamlessly merges
coding with immediate visual feedback.  Like the latter, EndBASIC offers
higher-level programming constructs and strong typing.

EndBASIC offers a simplified and restricted environment to learn the foundations
of programming and focuses on features that can quickly reward the programmer.
These features include things like a built-in text editor, commands to
manipulate the screen, and commands to interact with the hardware of a Raspberry
Pi.  Implementing this kind of features has priority over others such as
performance or a much richer language.

EndBASIC is written in Rust and runs both on the web and locally on a variety of
operating systems and platforms, including macOS, Windows, and Linux.

EndBASIC is free software under the [Apache 2.0 License](LICENSE).

## What's in this crate?

`endbasic` provides the command line binary for the EndBASIC language.  This
binary offers an interpreter for scripts as well as a fully-interactive REPL.
Launch the interpreter by typing `endbasic` and then type `HELP` inside it for
further guidance.

## Interactive usage

`endbasic` comes with a REPL command-line interface that offers fancy line
editing features and built-in documentation for all available features.  You can
start the interactive interface by typing:

```shell
endbasic
```

and you can exit it at any time by pressing CTRL+D.

The `HELP` command lets you obtain a list of all possible commands as well as
detailed usage information for any of them.

The interactive interface has support for manipulating a stored program: that
is, a program that lives in the memory of the interpreter.  To get started, use
the `EDIT` command to enter new lines into the program, `LIST` to inspect its
contents and `RUN` to execute it.  You can save and restore programs from disk
by using the `SAVE` and `LOAD` commands respectively, which by default are
backed by an `endbasic` subdirectory under your `Documents` folder.  You can
then inspect the contents of this directory with the `DIR` command.

The lines that `EDIT` prints during program editing are not meaningful to the
program and only exist to support interactive editing.  Line numbers are
typically multiples of ten to allow you to insert lines in-between others.  For
example: if your program previously had lines `20` and `30` but you forgot a
statement between those, you can do `EDIT 15` to enter that statement.  If you
run out of lines, or if you want to tidy up their numbers to be multiples of ten
again, use the `RENUM` command.

Finally, use `CLEAR` to erase all variables set in the interactive editor, and
`NEW` to also erase the stored program from memory and start afresh.

## Scripting usage

You can give `endbasic` the name of a program to execute:

```shell
endbasic some-program.bas
```

It is important to note that some features (like the `HELP` builtin command or
all stored program manipulation commands) are only available in the interactive
interface.

## Examples

You can peek into the `tests` subdirectory of this repository to see actual
EndBASIC code samples.  Look for all files with a `.bas` extension.
