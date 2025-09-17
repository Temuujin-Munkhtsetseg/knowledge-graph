pub const GET_DEFINITION_TOOL_NAME: &str = "get_definition";
pub const GET_DEFINITION_TOOL_DESCRIPTION: &str = r#"Go to definition for callable symbols (methods/functions) referenced on a specific line.

Behavior:
- Returns type "Definition" when the symbol is defined in the workspace.
- Returns type "ImportedSymbol" when the symbol is external (best-matching import statement).

Requirements:
- Provide the exact line from the file (whitespace preserved).
- Specify the callable symbol name you want to resolve.

Java example:
File: src/main/java/com/example/User.java
Line: var name = user.getFirstName() + user.getLastName();
Call:
{ "absolute_file_path": "/abs/path/to/src/main/java/com/example/User.java", "line": "var name = user.getFirstName() + user.getLastName();", "symbol_name": "getFirstName" }"#;
