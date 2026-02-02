# Better Auth Documentation Generator

Automatic documentation generator for Better Auth APIs and schemas.

## Overview

This crate provides tools to automatically generate comprehensive documentation from your Better Auth configuration.

## Features

- ✅ **API Documentation**: Generate OpenAPI/Swagger specs
- ✅ **Schema Documentation**: Document database schemas
- ✅ **Event Documentation**: List all available events
- ✅ **Webhook Documentation**: Webhook endpoint docs
- ✅ **Plugin Documentation**: Document installed plugins

## Quick Start

```rust
use better_auth_docs::DocumentationGenerator;

let docs = DocumentationGenerator::new(auth_context)
    .generate()
    .await?;

// Output as JSON
let json = docs.to_json()?;

// Output as Markdown
let markdown = docs.to_markdown()?;

// Output as HTML
let html = docs.to_html()?;
```

## See Also

- [Server](../server/README.md) - Main server application
