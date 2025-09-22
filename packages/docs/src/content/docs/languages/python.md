---
title: Python
description: High-level overview of Python support by gkg.
sidebar:
  order: 4
---

> 🚧 This functionality is under active development.

This document provides an overview of what is and isn't indexed by gkg for Python.

## Definitions

gkg indexes only callable definitions in Python—–definitions that can be invoked via function calls. Supported definitions include:

| Definition Type  | Example               |
| :--------------- | :-------------------- |
| **Class**        | `class MyClass: ...`  |
| **Function**     | `def foo(x): ...`     |
| **Named lambda** | `foo = lambda x: ...` |

### Limitations

The following callable definitions are not currently supported due to their infrequent use:

| Definition Type                     | Example                                                          |
| :---------------------------------- | :--------------------------------------------------------------- |
| **Dynamic classes**                 | `MyClass = type("MyClass", (object,), {})`                       |
| **Lambdas within data structures**  | `foo = {"bar": lambda x: ...}`                                   |
| **Module dictionary modifications** | `sys.modules[__name__].__dict__["new_function"] = lambda x: ...` |
| **Exec/Eval**                       | `exec("def foo(x): ...")`                                        |

## Imports

gkg indexes imported symbols as graph nodes. Supported import types:

| Import Type                  | Example                              |
| :--------------------------- | :----------------------------------- |
| **Import**                   | `import module`                      |
| **Aliased import**           | `import module as alias`             |
| **From import**              | `from module import symbol`          |
| **Aliased from import**      | `from module import symbol as alias` |
| **Wildcard import**          | `from module import *`               |
| **Relative import**          | `from .. import symbol`              |
| **Aliased relative import**  | `from .. import symbol as alias`     |
| **Relative wildcard import** | `from .. import *`                   |

## References

A reference is a _call_ to a function, class, or named lambda.

### Limitations

Perfect call graph generation is impossible for dynamic languages like Python. Therefore, the indexer has limitations in what references it can and cannot capture. Some of these limitations are also present in the Python LSP, and can be safely ignored, while others are under active development. If a limitation is marked with `(*)` then it is also present in the LSP.

#### Same-file references

Below are limitations for references to functions or classes defined in the _same_ file as the reference:

- **Inheritance:** Calls to inherited methods (e.g., `self.inherited_method()`) and `super()` calls are ignored.
- **(\*) Side-effects:** Function calls that modify parent scope values are ignored. For example, if `bar()` reassigns `foo` to `fizz`, subsequent `foo()` calls won't reference `fizz`. The same is true for calls to instance methods that modify the state of an object.
- **(\*) Dunder methods:** Magic method calls are ignored (e.g., `obj + other_obj` is interpreted by Python as `obj.__add__(other_obj)`, but this call is ignored).
- **(\*) Destructured assignments:** References after destructuring are ignored (e.g., `x()` where `(x, y) = (foo, bar)`). More generally, functions that are called via access to iterable data structures like lists, tuples, and dicts are ignored (e.g., `x[0]()` where `x[0] = foo`).
- **(\*) Function parameter calls:** Calls to functions passed as parameters are ignored. And calls to methods on objects that are passed as parameters.
- **(\*) Function output calls:** Calls to returned functions are ignored (e.g., `my_fn = foo(); my_fn()`).
- **(\*) Execution order:** References always point to the first definition encountered, regardless of redefinition timing. E.g. suppose `foo` is defined after `bar`, and it calls `bar`. If `bar` is redefined after `foo`, then the reference in `foo` will still resolve to the first `bar` instead of the second.
- **Getter/Setter methods:** Property (`@property`) getter and setter calls are ignored (e.g., Python interprets `obj.my_property = value` as a call to `my_property()`, but this is ignored).
- **(\*) Break-based control flow:** `for-else`, `while-else`, and `try-else` clauses aren't treated as control flow branches, even though they only execute if the loop doesn't hit a `break` statement. They are assumed to always execute. Therefore, any assignments or references made in an `else` are not marked as ambiguous.
- **(\*) Conditional class variables:** Conditionally assigned class variables (e.g., `my_fn = foo if condition else bar`) don't resolve references.
- **(\*) Dynamic attribute access::** Calls to dynamically accessed functions are ignored (e.g., `getattr(obj, "my_method")()` or `obj.__dict__["my_method"]()`).

#### Across-file references

Below are limitations for references to functions or classes that are defined in a _different_ file from the reference:

- **Conditional definitions/imports:** References to conditionally defined functions or classes are ignored (e.g., `if condition: def foo(): ... else: def foo(): ...`). Only one definition will be captured, though this is an extremely rare case in practice.
- **(\*) Definition aliasing:** References to aliased definitions are ignored (e.g., `def foo(): ...; bar = foo` when `bar` is imported and referenced in another file).
- **Wildcard imports:** References to definitions imported via wildcards are ignored (e.g., `from module import *`).
