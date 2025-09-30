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

The parser should identify and extract the following types of references:

| Reference Type                      | Example                               |
| :---------------------------------- | :------------------------------------ |
| **Function Call**                   | `myFunction()`                        |
| **Class/Object Instantiation**      | `MyClass()`                           |
| **Method/Callable Reference**       | `::myFunction`, `MyClass::myProperty` |
| **Enum Entry Reference**            | `Color.RED`                           |
| **Operator Function Call**          | `a.plus(b)`, `a.times(b)`, `a + b`    |
| **Type inference from if/when/try** | `val x = if (c) a else b`             |

### Limitations

- **Generic type resolution**: The parser does not resolve or track generic type parameters and their instantiations. This includes generic types such as `List<T>`, `Map<K, V>`, and arrays (e.g., `Array<T>`).
- **Standard library method and collection type resolution**: Types of standard library methods and properties, as well as standard collections operations on arrays and maps, are not resolved.
- **Chains with lambdas**: Resolving references within chained calls that include lambda expressions (e.g., `list.filter { ... }.map { ... }`) is limited.
- **Property references**: Since the indexer focuses on building a call graph, references to properties (field accesses) are not tracked as edges in the graph.

### Under the hood

The Kotlin indexer's architecture is designed to handle Kotlin's static, type-safe nature effectively. It uses a two-phase strategy to build a comprehensive map of the codebase before connecting the dots.

#### Implementation Architecture

The logic is divided into two phases:

1.  **Indexing:** The `KotlinAnalyzer` first performs a global analysis of all Kotlin files in the project. It builds a comprehensive set of indexes containing every definition (class, method, field) and imports. This phase does not resolve any expressions, it only catalogs what exists and where.
2.  **Resolution:** After the indexes are complete, the `ExpressionResolver` processes the expressions (method calls, field accesses, etc.) from each file. Using the global indexes, it resolves these expressions to their precise definitions, traversing class hierarchies and scopes to create the final edges.

#### Phase 1: Global Index Construction

Before resolving any references, the indexer builds several key indexes to enable fast and accurate lookups during the resolution phase.

1.  **Initial Population:** The `KotlinAnalyzer` iterates through every Kotlin file. For each definition, it creates a `DefinitionNode` and stores it in several indexes within the `ExpressionResolver`:
    - `definition_nodes`: An `FxHashMap<String, DefinitionNode>` for direct FQN lookups.
    - `package_files`: An `FxHashMap<String, Vec<String>>` mapping a package to the file paths that belong to it.
2.  **File-Level Scopes:** For each file, a `KotlinFile` struct is created, which builds a tree of scopes (from package down to methods and blocks). Each scope tracks the local variables, parameters, and fields available within it.

#### Phase 2: Expression Resolution

This is where the semantic analysis happens. The `ExpressionResolver` iterates through each reference from every file and uses the following process:

1.  **Get Context:** The reference's location within a file is used to determine its starting scope.
2.  **Recursive Resolution:** The resolver traverses the expression tree. For a chained call like `service.repository.findUser()`, it works left-to-right:
    - **Resolve `service`:** It looks up the identifier `service` by walking up the scope tree from the reference's location. This search checks local variables, method parameters, and class fields until a definition is found. The type of `service` is then determined (e.g., `com.example.UserService`).
    - **Resolve `.repository`:** Now with the context of `com.example.UserService`, it resolves the `repository` field access. The resolver looks for a `repository` field within the `UserService` class. If not found, it walks up the class's inheritance hierarchy (superclasses and interfaces). The return type of this field becomes the new context (e.g., `com.example.UserRepository`).
    - **Resolve `.findUser()`:** With the context of `com.example.UserRepository`, it performs a method lookup for `findUser()`. This lookup also traverses the class hierarchy.
3.  **Create Graph Edge:** If the entire expression is resolved successfully, a `Calls` edge is created in the graph from the containing method to the final resolved method (`findUser`).

##### Contextual Resolution

When a direct resolution for a member function call fails, the `ExpressionResolver` attempts to find a match using contextual clues. This fallback mechanism is particularly useful for resolving extension functions or functions on generic types. The process is as follows:

1.  **Check Generics:** The resolver first inspects the generic type parameters available in the current scope. It checks if any of the generic types have a member function matching the call.
2.  **Search Imports and Packages:** If generics do not yield a match, the resolver consults its global function registry. It searches for top-level functions that match the name and could be applicable. This search includes:
    - Functions in explicitly imported files.
    - Functions available through wildcard imports.
    - Functions defined in the same package.

This contextual resolution allows the indexer to connect calls to extension functions or resolve calls on objects whose types are generic at that point in the code.

> **Note:** This contextual resolution approach may occasionally create invalid edges when the resolver matches a function that isn't actually applicable in the given context. However, we believe it will yield better overall results for code navigation and understanding, even with the trade-off of some false positives.

#### Kotlin Type and Method Resolution

1.  **Type Resolution:** To resolve a type name (e.g., `List`), the resolver checks in this order:
    1.  Classes defined in the same file (e.g., inner classes).
    2.  Explicitly imported classes (`import kotlin.collections.List`).
    3.  Wildcard imports (`import kotlin.collections.*`).
    4.  Classes in the same package.
2.  **Method & Field Lookup:** When resolving a method or field on a type, a resolver implements Kotlin's inheritance rules:
    1.  It first checks the class itself for the member.
    2.  If not found, it recursively searches the superclass.
    3.  Then, it recursively searches all implemented interfaces.

## Out of scope

The following Kotlin features are currently **out of scope** for the Knowledge Graph:

- **Dynamic/Reflection-based Calls**: Calls resolved at runtime using reflection.
- **Complex DSLs**: Resolving references within highly custom Domain Specific Languages.
- **Multi-platform Project Resolution**: Resolving across different source sets.
- **Annotation Processing**: Semantic meaning from annotation processors (e.g., kapt) is not included.
- **Generated Code**: Code generated at compile time (e.g., via codegen plugins or compiler plugins) is not included.
- **Complex Type Inference**: Deep semantic type inference for generics or lambdas.
- **Anonymous Object Expressions**: Resolution for members in anonymous objects is limited.
