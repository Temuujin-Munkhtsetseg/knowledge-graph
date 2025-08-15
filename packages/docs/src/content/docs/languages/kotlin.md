---
title: Kotlin
description: High-level overview of Kotlin support by gkg.
sidebar:
  order: 3
---

> 🚧 This functionality is under active development.

This document provides an overview of what Kotlin code is indexed and what is not yet supported.

## Definitions

| Definition Type        | Example                              |
| :--------------------- | :----------------------------------- |
| **Class**              | `class MyClass`                      |
| **Interface**          | `interface Repository<T>`            |
| **Function**           | `fun myFunction()`                   |
| **Property**           | `val myProperty: Int`                |
| **Companion Object**   | `companion object`                   |
| **Constructor**        | `constructor()`                      |
| **Data Class**         | `data class User`                    |
| **Value Class**        | `value class UserId(val id: String)` |
| **Sealed Class**       | `sealed class Result`                |
| **Object**             | `object MyObject`                    |
| **Top-Level Function** | `fun main()`                         |
| **Top-Level Property** | `val VERSION = "1.0"`                |
| **Annotation Class**   | `annotation class MyAnnotation`      |
| **Enum Class**         | `enum class Color`                   |
| **Enum Entry**         | `RED, GREEN, BLUE`                   |

### Limitations

| Definition Type                           | Example                                            |
| :---------------------------------------- | :------------------------------------------------- |
| **Method Overloading**                    | `fun add(a: Int)` and `fun add(a: String)`         |
| **Constructor Overloading**               | `constructor(a: Int)` and `constructor(a: String)` |
| **Non-callable Lambda and their content** | `{ val x = 1; val y = 2 }`                         |

### Missing Definitions

| Definition Type | Example                     |
| :-------------- | :-------------------------- |
| **Type Alias**  | `typealias UserId = String` |

## Imports

| Import Type         | Example                          |
| :------------------ | :------------------------------- |
| **Import**          | `import kotlin.collections.List` |
| **Wildcard Import** | `import kotlin.collections.*`    |
| **Aliased Import**  | `import foo.Bar as Baz`          |

## References

> 🚧 **Cross-file and reference support for Kotlin is currently in development. The progress can be tracked [in this issue](https://gitlab.com/gitlab-org/rust/knowledge-graph/-/issues/156).**

The parser should identify and extract the following types of references **within the same file**:

| Reference Type                 | Example                               |
| :----------------------------- | :------------------------------------ |
| **Function Call**              | `myFunction()`                        |
| **Property Access**            | `myClass.myProperty`                  |
| **Class/Object Instantiation** | `MyClass()`                           |
| **Chain of Calls/Properties**  | `foo.bar().baz.property`              |
| **Method/Callable Reference**  | `::myFunction`, `MyClass::myProperty` |
| **Enum Entry Reference**       | `Color.RED`                           |
| **Operator Function Call**     | `a.plus(b)`, `a.times(b)`             |

### Limitations

- **Operator functions**: Operator function calls using signs (e.g., `+`, `-`, `/`, etc.) will not be resolved to their underlying function definitions.
- **Generic type resolution**: The parser does not resolve or track generic type parameters and their instantiations. This includes generic types such as `List<T>`, `Map<K, V>`, and arrays (e.g., `Array<T>`).
- **Standard library method and collection type resolution**: Types of standard library methods and properties, as well as standard collections operations on arrays and maps, are not resolved.
- **Complex type inference**: The parser does not perform deep or advanced type inference, especially for generics, lambdas, or type projections.
- **Chains with lambdas**: Resolving references within chained calls that include lambda expressions (e.g., `list.filter { ... }.map { ... }`) is limited.
- **Casting in expressions**: Casting (using `as`, `as?`, or type conversion functions) that occurs in the middle of a function, inside a `when` expression, or within an `if` statement is not supported.

## Out of scope

The following Kotlin features are currently **out of scope** for the Knowledge Graph:

- **Dynamic/Reflection-based Calls**: Calls resolved at runtime using reflection.
- **Complex DSLs**: Resolving references within highly custom Domain Specific Languages.
- **Multi-platform Project Resolution**: Resolving across different source sets.
- **Annotation Processing**: Semantic meaning from annotation processors (e.g., kapt) is not included.
- **Generated Code**: Code generated at compile time (e.g., via codegen plugins or compiler plugins) is not included.
- **Complex Type Inference**: Deep semantic type inference for generics or complex expressions/lambdas.
- **Anonymous Object Expressions**: Resolution for members in anonymous objects is limited.
