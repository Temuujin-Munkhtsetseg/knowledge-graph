---
title: Ruby
description: High-level overview of Ruby support by the gkg.
sidebar:
  order: 1
---

> 🚧 This functionality is under active development.

This document covers what Ruby code gets indexed into the Knowledge Graph and what isn't supported yet.

## Definitions

This table represents what Definitions and their FQNs (Fully Qualified Names) are captured into the Graph.

| Definition Type      | Example                         | FQN Example                        |
| :------------------- | :------------------------------ | :--------------------------------- |
| **Class**            | `class User`                    | `User`                             |
| **Module**           | `module Authentication`         | `Authentication`                   |
| **Method**           | `def save`                      | `User#save`                        |
| **Singleton Method** | `def self.find_by_email(email)` | `User::find_by_email`              |
| **Lambda**           | `lambda { \|x\| x \* 2 }`       | `User#validate_data`               |
| **Proc**             | `proc { puts "Hello" }`         | `NotificationService::log_message` |

### Limitations

Dynamic method creation is not captured as definitions at this time.

| Definition Type             | Example                                |
| :-------------------------- | :------------------------------------- |
| **Dynamic Method Creation** | `define_method(:dynamic_name) { ... }` |
| **Method Missing Handlers** | `def method_missing(name, *args)`      |
| **Eval-based Definitions**  | `eval("def #{name}; end")`             |

### Contextual Elements (Not Captured as Definitions)

| Element Type           | Example             | Notes                                                    |
| :--------------------- | :------------------ | :------------------------------------------------------- |
| **Constants**          | `VERSION = "1.0.0"` | Used for FQN context, not as callable definitions        |
| **Instance Variables** | `@user_name`        | Not captured as definitions, but used for type inference |
| **Class Variables**    | `@@count`           | Not captured as definitions, but used for type inference |
| **Global Variables**   | `$global_var`       | Not captured as definitions, but used for type inference |

## Imports

We capture the following import types:

| Import Type          | Example                             |
| :------------------- | :---------------------------------- |
| **Require**          | `require 'json'`                    |
| **Require Relative** | `require_relative './user_service'` |
| **Load**             | `load 'config.rb'`                  |
| **Gem Dependencies** | `gem 'rails', '~> 7.0'`             |

### Limitations

The following import types are not captured as definitions at this time:

| Import Type              | Example                               |
| :----------------------- | :------------------------------------ |
| **Autoloading**          | Rails' automatic class loading        |
| **Dynamic Requires**     | `require(file_name_variable)`         |
| **Conditional Requires** | `require 'gem' if defined?(GemClass)` |

## References

Ruby reference resolution provides cross-file analysis with type inference:

### Currently Resolved

| Reference Type               | Resolution Level        | Example                                |
| :--------------------------- | :---------------------- | :------------------------------------- |
| **Direct Method Calls**      | Full resolution         | `User.find_by_email("test@test.com")`  |
| **Instance Method Chains**   | Full resolution         | `user.profile.update(name: "new")`     |
| **Variable Assignments**     | Type tracking           | `user = User.new; user.save`           |
| **Cross-file References**    | Full resolution         | Method calls spanning multiple files   |
| **Inheritance Resolution**   | Full resolution         | Finding methods in superclasses        |
| **Module Inclusion Lookup**  | Full resolution         | Methods from included/extended modules |
| **Constant Resolution**      | Full resolution         | `User::ADMIN_ROLE`                     |
| **Instance Variable Access** | Heuristic inference     | `@user.profile` (infers User type)     |
| **Singleton Method Calls**   | Full resolution         | `NotificationService.notify`           |
| **Block Method Calls**       | Within-block resolution | `users.each { \|u\| u.activate! }`     |

### Limitations

- **Dynamic Method Dispatch**: `obj.send(:method_name)` calls cannot be resolved
- **Method Missing Handlers**: Calls that trigger `method_missing` are not tracked
- **Complex Polymorphism**: Without explicit type annotations, some polymorphic calls fail
- **Metaprogramming**: Runtime-generated methods via `define_method` or `eval`
- **Framework Magic**: Some Rails methods (ActiveRecord query methods, etc.) may not resolve

## Out of scope

The following Ruby features are currently **out of scope** for the Knowledge Graph:

- **Dynamic Method Dispatch**: Code using `send`, `public_send`, or `method` for method calls
- **Runtime Metaprogramming**: Methods defined via `define_method`, `class_eval`, or `instance_eval`
- **Method Missing Magic**: Classes that heavily rely on `method_missing` for API design
- **Reflection-based Calls**: Using Ruby's reflection APIs for method invocation
- **String Evaluation**: Code execution via `eval`, `instance_eval`, or `class_eval` with strings
- **Complex DSLs**: Domain-specific languages that modify method resolution
- **Monkey Patching**: Runtime modifications to existing classes from external gems
- **Autoloading Magic**: Framework-specific automatic class loading (Rails autoloader)
- **Binding-based Evaluation**: Code execution using `Binding` objects

Code that relies heavily on these features won't be fully represented in the Knowledge Graph at this time.

## Under the hood

### Implementation Architecture

The Ruby indexer implements **Expression-Oriented Type Inference**, a two-phase strategy inspired by implementations in modern LSPs (like [`ruby-lsp`](https://github.com/Shopify/ruby-lsp)) and prior art with other our other language parsers.

The logic is divided into two phases:

1. **Parser (`gitlab-code-parser`):** Performs purely structural analysis. It parses the code and extracts definitions and _unresolved expressions_. It deconstructs complex statements like `user = User.find(123).name` into a structured representation but does **not** attempt to determine what `User` is or where `.name` is defined.
2. **Indexer (`knowledge-graph`):** Performs semantic analysis. With the complete set of definitions and expressions from the entire project, it connects the dots, infers types, and resolves the references to create the final edge list for the Knowledge Graph.

#### Phase 1: Global Definition and Hierarchy Map Construction

Before resolving any references, the indexer first builds a complete map of all definitions and how they relate to each other across the project.

1.  **Initial Population:** The `RubyAnalyzer` iterates through the `FileProcessingResult` of every Ruby file. For each `RubyDefinitionInfo` found, it's inserted into a `DefinitionMap` using its FQN as the key.
2.  **Hierarchy Resolution:** Once all definitions are cataloged, the indexer processes the `DefinitionMap` again. For each class or module, the indexer resolves its superclass and any included/prepended/extended module names against the now-complete map. This turns the flat list of definitions into a connected graph showing how classes and modules inherit from or include each other.

The `DefinitionMap` contains several indexes for fast lookups:

- `definitions`: `FxHashMap<Arc<str>, Arc<DefinitionNode>>` for direct FQN lookups.
- `instance_methods` / `singleton_methods`: Maps a class FQN to a `SmallVec` of its method names.
- `inheritance_chain`: Maps a class FQN to its parent's FQN, so we can walk up the inheritance tree.

#### Phase 2: Expression Resolution and Type Inference

This is where the type inference happens. The `ExpressionResolver` iterates through each `ReferenceInfo` from every file, using this process:

1.  **Get Context:** The reference's scope FQN and its `RubyExpressionMetadata` are retrieved.
2.  **Initialize State:** `let mut current_type_context: Option<FQN> = None;`
3.  **Iterate Symbol Chain:** For each `RubyExpressionSymbol` in the `metadata.symbols` vector (from left to right):
    - **Resolve the Symbol:**
      - **First symbol:**
        - `Identifier` (e.g., `user`): Looked up in the `TypeMap`. If not found, it could be an implicit call on `self`, making the context the type of the current scope.
        - `Constant` (e.g., `User`): Looked up in the `DefinitionMap` as contextual elements for FQN resolution.
        - `InstanceVariable` (`@user`): The context is the type of `self` for the current scope.
        - `ClassVariable` (`@@count`): The context is the FQN of the enclosing class/module.
      - **Subsequent symbol (`MethodCall`):**
        - The `current_type_context` must be set from the previous step.
        - A **Method Lookup** for the symbol's name is performed against the `current_type_context`. This attempts to mimic Ruby's method lookup order.
    - **Infer Return Type and Update Context:**
      - After resolving a method call, its return type is inferred to become the `current_type_context` for the _next_ symbol in the chain. This uses a prioritized set of heuristics.
4.  **Create Graph Edge:**
    - If the entire symbol chain was resolved, a `DefinitionRelationship` edge is created in the graph from the reference's location to the target definition's location.
5.  **Update `TypeMap`:**
    - If the reference was an assignment (`user = User.new`), the final resolved `current_type_context` from the RHS is stored in the `TypeMap`.

### Type Inference Heuristics

The resolver uses a prioritized set of heuristics to infer the return type of a method call, which becomes the context for the _next_ symbol in an expression chain:

- **Convention-based:** A call to `.new` on a class is known to return an instance of that class. Similarly, common Rails methods like `find`, `first`, or `last` on an ActiveRecord model are assumed to return an instance of that model.
- **Default:** If no heuristic matches, the return type cannot be confidently determined, and the resolution chain stops, resulting in a **partial resolution**.
- **Instance Variable Naming:** Heuristics are used to infer types from instance variable names, such as `@user` being inferred as `User` and `@notification_service` as `NotificationService`.

The following heuristics supported by other lsp implementations are not yet implemented:

- **YARD Doc Parsing:** The indexer can be enhanced to parse YARD comments, specifically the `@return [ClassName]` tag. This provides explicit type information.
- **Assignment Tracking:** For an assignment like `user = User.new`, the type of the `user` variable is tracked within its scope, making it available for resolving subsequent calls like `user.save`.

### Ruby Method Lookup Implementation

The resolver implements Ruby's precise method lookup order, which is critical for accurate reference resolution in complex Ruby codebases. This mirrors how the Ruby VM performs method dispatch at runtime.

#### Method Resolution Order

Ruby's method lookup follows a specific hierarchy:

1. **Singleton methods** on the class itself (e.g., `User.find`)
2. **Instance methods** on the class itself (e.g., `def save` in `User`)
3. **Included modules** in reverse order of inclusion (last included = first searched)
4. **Superclass methods** following the same pattern recursively up the inheritance chain
5. **BasicObject** as the ultimate ancestor

The resolver attempts to handle several complex Ruby scenarios:

**Module Inclusion Order**: When multiple modules are included, Ruby searches them in reverse order of inclusion. If a class includes `ModuleA` then `ModuleB`, method lookup searches `ModuleB` first, then `ModuleA`.

```ruby
class User
  include Authenticatable  # Included first
  include Trackable       # Included second - searched first

  # Method lookup order: User → Trackable → Authenticatable → ApplicationRecord
end
```

**Prepend vs Include**: The resolver distinguishes between `prepend` and `include`. Prepended modules are inserted before the class in the lookup chain, while included modules come after.

**Singleton Class Hierarchy**: Class methods (`def self.method`) are stored in the singleton class, which has its own inheritance chain. The resolver maintains separate hierarchies for instance and singleton methods.

**Method Visibility**: The resolver respects method visibility (`private`, `protected`, `public`) and calling context. A `private` method can only be called without an explicit receiver or with `self`.

### Limitations

Since Ruby is a dynamic language and the indexing is done through pure static analysis, some patterns can't be resolved. The ruby implementation currently has these limitations:

- **Dynamic Method Invocation:** Calls using `send`, `public_send`, or `instance_eval`, where the method name is a variable.
- **Metaprogramming:** Methods and classes created at runtime via `define_method` or `method_missing`.
- **Arbitrary Polymorphism (Duck Typing):** Cannot determine the correct definition when a method is called on a variable that could hold objects of multiple, unrelated types.
- **Monkey Patching / Open Classes:** Cannot determine the correct definition when a method has been redefined in a file loaded at runtime, as the load order is unknown.
- **Complex Return Type Inference:** Resolution of method chains will fail if a method's return type cannot be determined through heuristics or YARD documentation.
- **Dynamic `require` Statements:** Cannot resolve module loads where the path is dynamically constructed from a variable.
- **Code Execution via `eval`:** Any code defined or executed within an `eval` string is completely invisible.
- **Project-Specific Load Path Modifications:** Does not understand custom `$LOAD_PATH` configurations that alter how `require` resolves file paths.

We intend to address edge cases and unsupported patterns as we iterate on the implementation.
