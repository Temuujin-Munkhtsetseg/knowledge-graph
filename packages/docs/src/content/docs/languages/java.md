---
title: Java
description: High-level overview of Java support by the gkg.
sidebar:
  order: 2
---

> 🚧 This functionality is under active development.

This document provides an overview what Java code is indexed and what is not yet supported.

## Definitions

| Definition Type            | Example                             |
| -------------------------- | ----------------------------------- |
| **Class**                  | `public class MyClass`              |
| **Interface**              | `public interface Repository<T>`    |
| **Enum**                   | `public enum Status`                |
| **Enum Constant**          | `ACTIVE("active")`                  |
| **Record**                 | `public record Person(String name)` |
| **Annotation**             | `public @interface MyAnnotation`    |
| **Annotation Declaration** | `String value() default "";`        |
| **Method**                 | `public void myMethod()`            |
| **Constructor**            | `public MyClass()`                  |
| **Record Constructor**     | `public Person()`                   |
| **Record access methods**  | `personRecord.name()`               |

### Limitations

| Definition Type           | Example                                          |
| ------------------------- | ------------------------------------------------ |
| **Method Overloads**      | `void method(int a)` and `void method(String a)` |
| **Constructor Overloads** | `MyClass(int a)` and `MyClass(String a)`         |

## Imports

| Import Type         | Example                            |
| ------------------- | ---------------------------------- |
| **Import**          | `import java.util.List;`           |
| **Static Import**   | `import static java.lang.Math.PI;` |
| **Wildcard Import** | `import java.util.*;`              |

## References

| Reference Type                 | Example                                              |
| :----------------------------- | :--------------------------------------------------- |
| **Function Call**              | `myFunction()`                                       |
| **Static Function Call**       | `MyClass.staticMethod()`                             |
| **Class/Object Instantiation** | `new MyClass()`                                      |
| **Object Array Creation**      | `new MyClass[1]`                                     |
| **Chain of Calls/Properties**  | `foo.bar().baz.property`                             |
| **Method/Callable Reference**  | `::myFunction`, `MyClass::myProperty`                |
| **Annotations**                | `@Annotation`, `@Annotation(...)`                    |
| **this Reference**             | `this.method()`                                      |
| **super Reference**            | `super.method()`                                     |
| **Pattern Variable Calls**     | `obj instanceof MyClass myClass`, `myClass.method()` |

### Limitations

- **Generic type resolution**: The Knowledge Graph does not resolve or track generic type parameters and their instantiations. This includes generic types such as `List<T>`, `Map<K, V>`, and arrays (e.g., `Array<T>`).
- **Standard library method and collection type resolution**: Types of standard library methods and properties, as well as standard collections operations on arrays and maps, are not resolved.
- **Complex type inference**: The parser does not perform deep or advanced type inference, especially for generics, lambdas, or type projections.
- **Chains with lambdas**: Resolving references within chained calls that include lambda expressions (e.g., `list.filter((i) -> ...).map((i) -> ...)`) is limited.

### Under the hood

The Java indexer's architecture is designed to handle Java's static, type-safe nature effectively. It uses a two-phase strategy to build a comprehensive map of the codebase before connecting the dots.

#### Implementation Architecture

The logic is divided into two phases:

1.  **Indexing:** The `JavaAnalyzer` first performs a global analysis of all Java files in the project. It builds a comprehensive set of indexes containing every definition (class, method, field) and imports. This phase does not resolve any expressions; it only catalogs what exists and where.
2.  **Resolution:** After the indexes are complete, the `ExpressionResolver` processes the expressions (method calls, field accesses, etc.) from each file. Using the global indexes, it resolves these expressions to their precise definitions, traversing class hierarchies and scopes to create the final edges for the Knowledge Graph.

#### Phase 1: Global Index Construction

Before resolving any references, the indexer builds several key indexes to enable fast and accurate lookups during the resolution phase.

1.  **Initial Population:** The `JavaAnalyzer` iterates through every Java file. For each definition, it creates a `DefinitionNode` and stores it in several maps within the `ExpressionResolver`:
    - `definition_nodes`: An `FxHashMap<String, DefinitionNode>` for direct FQN lookups.
    - `declaration_files`: An `FxHashMap<String, String>` mapping an FQN to the file path where it's declared.
    - `package_class_index`: A `FxHashMap<String, FxHashMap<String, String>>` to quickly find a class file within a given package.
2.  **File-Level Scopes:** For each file, a `JavaFile` struct is created, which builds a tree of scopes (from package down to methods and blocks). Each scope tracks the local variables, parameters, and fields available within it. This is crucial for resolving identifiers later.

#### Phase 2: Expression Resolution

This is where the semantic analysis happens. The `ExpressionResolver` iterates through each reference from every file and uses the following process:

1.  **Get Context:** The reference's location within a file is used to determine its starting scope.
2.  **Recursive Resolution:** The resolver traverses the expression tree. For a chained call like `service.repository.findUser()`, it works left-to-right:
    - **Resolve `service`:** It looks up the identifier `service` by walking up the scope tree from the reference's location. This search checks local variables, method parameters, and class fields until a definition is found. The type of `service` is then determined (e.g., `com.example.UserService`).
    - **Resolve `.repository`:** Now with the context of `com.example.UserService`, it resolves the `repository` field access. The resolver looks for a `repository` field within the `UserService` class. If not found, it walks up the class's inheritance hierarchy (superclasses and interfaces). The return type of this field becomes the new context (e.g., `com.example.UserRepository`).
    - **Resolve `.findUser()`:** With the context of `com.example.UserRepository`, it performs a method lookup for `findUser()`. This lookup also traverses the class hierarchy.
3.  **Create Graph Edge:** If the entire expression is resolved successfully, a `Calls` edge is created in the graph from the containing method to the final resolved method (`findUser`).

#### Java Type and Method Resolution

1.  **Type Resolution:** To resolve a type name (e.g., `List`), the resolver checks in this order:
    1.  Classes defined in the same file (e.g., inner classes).
    2.  Explicitly imported classes (`import java.util.List;`).
    3.  Wildcard imports (`import java.util.*;`).
    4.  Classes in the same package.
2.  **Method & Field Lookup:** When resolving a method or field on a type, a resolver implements Java's inheritance rules:
    1.  It first checks the class itself for the member.
    2.  If not found, it recursively searches the superclass.
    3.  Then, it recursively searches all implemented interfaces.

## Out of scope

The following Java features are currently **out of scope** for the Knowledge Graph:

- **Reflection**: Code that uses Java Reflection APIs (e.g., `Class.forName`, `Method.invoke`) to inspect or modify program structure at runtime is indexed.
- **Module System (JPMS)**: Java Platform Module System constructs (`module-info.java`, `requires`, `exports`, etc.) are not represented in the graph.
- **Dynamic Proxy Classes**: Classes generated at runtime via `java.lang.reflect.Proxy` or similar mechanisms are not indexed.
- **Annotation Processing**: Code generated or modified by annotation processors during compilation is not included.
- **Bytecode Manipulation**: Any entities or relationships introduced via bytecode manipulation libraries (e.g., ASM, Javassist) are not captured.
- **Lambdas and Method References**: While basic lambda expressions may be partially represented, advanced usages and method references may not be fully indexed.
- **Generated Code**: Code generated by tools (e.g., Lombok, AutoValue) is not guaranteed to be indexed unless present in the source tree.
- **External Dependencies**: References to classes, methods, or fields from external dependencies (such as Maven or Gradle artifacts) may not be fully resolved or represented in the graph, especially if their source code is not available.

If your code relies heavily on these features, the Knowledge Graph may not provide a complete representation.
