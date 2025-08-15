---
title: TypeScript / JavaScript
description: High-level overview of TypeScript and JavaScript support by the gkg.
sidebar:
  order: 2
---

> 🚧 This functionality is under active development.

This document provides an overview what TypeScript and JavaScript code is indexed and what is not yet supported.

## Definitions

| Definition Type                   | Example                                   |
| --------------------------------- | ----------------------------------------- |
| **Class**                         | `class NotificationProcessor`             |
| **Named Class Expression**        | `const UserClass = class {}`              |
| **Method**                        | `async processNotification()`             |
| **Private Method**                | `#validateConfig(config)`                 |
| **Static Field**                  | `static InnerStatic = class {}`           |
| **Function**                      | `function generateUniqueId()`             |
| **Named Arrow Function**          | `const validate = (input) => {}`          |
| **Named Function Expression**     | `const validate = function() {}`          |
| **Named Generator Function**      | `const gen = function* () {}`             |
| **Enums**                         | `enum Direction { Up, Down }`             |
| **Class Factory Functions**       | `const ClassFactory = (base) => class {}` |
| **Dynamic Method Names**          | `[`dynamic${Date.now()}`]() {}`           |
| **Named IIFEs**                   | `(function namedIIFE() {})()`             |
| **Interface Method Declarations** | `interface Name { methodName(): void }`   |

### Limitations

| Definition Type               | Example                                 |
| ----------------------------- | --------------------------------------- |
| **Object Methods**            | `const obj = { method() {} }`           |
| **Getters/Setters**           | `get name() {}` or `set name(value) {}` |
| **Decorators**                | `@decorator class MyClass`              |
| **Complex Class Expressions** | `const classes = { A: class A {} }`     |
| **Ambient Declarations**      | `declare class ExternalClass {}`        |

## Imports

| Import Type                      | Example                                      |
| -------------------------------- | -------------------------------------------- |
| **Default Import**               | `import React from 'react'`                  |
| **Named Import**                 | `import { useState } from 'react'`           |
| **Aliased Import**               | `import { Component as ReactComponent }`     |
| **Mixed Imports**                | `import D, { A, B as C } from 'mod'`         |
| **Namespace Import**             | `import * as React from 'react'`             |
| **Side-Effect Import**           | `import 'reflect-metadata'`                  |
| **CommonJS Require**             | `const fs = require('fs')`                   |
| **Destructured Require**         | `const { readFile } = require('fs')`         |
| **Aliased Destructured Require** | `const { readFile: fsRead } = require('fs')` |
| **Dynamic Import**               | `const fs = await import('fs')`              |
| **TypeScript Import Require**    | `import express = require('express')`        |
| **Type-Only Imports**            | `import type { FC } from 'react'`            |

### Limitations

| Import Type                         | Example                                                      |
| ----------------------------------- | ------------------------------------------------------------ |
| **Re-exports**                      | `export { Component } from 'react'`                          |
| **Dynamic Import with Expressions** | `const module = await import(moduleName)`                    |
| **Import Assertions**               | `import config from './config.json' assert { type: 'json' }` |

## References

Broad coverage of common patterns for immediate utility, trading perfect accuracy for rapid knowledge graph population. Reference resolution currently follows a pragmatic "80% solution" approach.

### Currently Resolved

| Reference Type            | Resolution Level    | Example                               |
| ------------------------- | ------------------- | ------------------------------------- |
| **Simple Function Calls** | Basic name matching | `generateUniqueId('prefix')`          |
| **Method Calls**          | Basic name matching | `processor.queueNotification()`       |
| **Constructor Calls**     | Basic name matching | `new EmailNotificationHandler()`      |
| **This Calls**            | Basic name matching | `this.emailService.sendEmail()`       |
| **Super Calls**           | Basic name matching | `super.postProcess(notification)`     |
| **Async/Await Calls**     | Basic name matching | `await this.emailService.sendEmail()` |
| **Property Access Calls** | Basic name matching | `notification.payload.to()`           |

### Tracked but Not Resolved

| Reference Type                  | Example                                                      |
| ------------------------------- | ------------------------------------------------------------ |
| **Array/Object Indexing Calls** | `this.processingQueue[varName]()`                            |
| **Member Expression Chains**    | `this.emailService.sendEmail().then()`                       |
| **Assignment Tracking**         | `const handler = factory.createHandler(); handler.process()` |
| **Enum Access**                 | `Direction.Up`                                               |
| **Import Usage**                | `import { api } from './api'; api.call()`                    |

### Limitations (Aiming to resolve most by GitLab 18.4)

- **No scope awareness**: Variables with same names in different scopes treated identically
- **No variable shadowing**: Local declarations don't override outer scope correctly
- **No context awareness**: `this.method()` calls not resolved based on containing class
- **No assignment tracking**: `x = new Service(); x.method()` doesn't link to Service class
- **No import resolution**: Imported symbols never connected to their usage
- **No property resolution**: Object property access never resolved to definitions
- **Type System Resolution**: TypeScript type information, interfaces, and type aliases are not used for reference resolution

## Out of scope

The following TypeScript/JavaScript features are currently **out of scope** for the Knowledge Graph:

- **Module Namespace Resolution**: `import * as Utils from './utils'; Utils.helper()` namespace access is not resolved
- **Prototype Chain Resolution**: `MyClass.prototype.method.call()` and prototype-based inheritance patterns
- **Hoisted Function References**: Calling functions before their declaration (JavaScript hoisting behavior)
- **Control Flow Analysis**: Variable assignments within conditional blocks affecting resolution
- **Closure Resolution**: Inner functions accessing outer scope variables in complex closure scenarios
- **Generic Type Resolution**: TypeScript generic function and class usage patterns
- **Callback Context Resolution**: `array.map(item => item.process())` where method resolution depends on callback context

If your code relies heavily on one of the limitations or out of scope features, the Knowledge Graph may not provide a complete representation of symbol relationships.

For future releases, starting with GitLab 18.4, we aim to resolve most of the limitations and out of scope features. Progress will be tracked in the [(JS/TS) Parse Intra-file References - Scope Aware Resolution](https://gitlab.com/gitlab-org/rust/gitlab-code-parser/-/issues/97) issue.
