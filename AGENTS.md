# AGENTS.md

This file provides guidance for agentic coding agents working with the GPUI Component codebase.

## Build, Lint, and Test Commands

### Core Commands
```bash
# Build the project
cargo build

# Run the Story Gallery (component showcase)
cargo run

# Run specific examples
cargo run --example hello_world
cargo run --example table

# Lint checking (REQUIRED)
cargo clippy -- --deny warnings

# Format checking (REQUIRED)
cargo fmt --check

# Spell checking
typos

# Check for unused dependencies
cargo machete
```

### Testing Commands
```bash
# Run all tests (NOTE: Tests are per user configuration - don't need to run)
cargo test --all

# Run tests for specific crate
cargo test -p gpui-component

# Run single test
cargo test test_name

# Run doc tests
cargo test -p gpui-component --doc

# Run test with filter
cargo test -- test_prefix
```

### Performance Profiling
```bash
# View FPS on macOS
MTL_HUD_ENABLED=1 cargo run

# Profile performance using Samply
samply record cargo run
```

## Code Style Guidelines

### Imports and Dependencies
- Use workspace dependencies from root `Cargo.toml`
- Organize imports: std imports → external crates → local modules
- Prefer `use crate::` for internal imports
- Use `use gpui::{...}` for GPUI imports

### Naming Conventions
- **Types**: PascalCase (e.g., `ButtonVariant`, `SwitchState`)
- **Functions**: snake_case (e.g., `new_button`, `set_checked`)
- **Constants**: SCREAMING_SNAKE_CASE
- **File organization**: Group related components in directories (e.g., `button/` with `mod.rs`, `button.rs`, etc.)

### Component Architecture
- Components should implement `RenderOnce` trait for stateless design
- Use builder pattern with fluent API (`.checked(true)`, `.primary()`, `.size(Size::Large)`)
- First parameter of constructors should be `id: impl Into<ElementId>`
- Default size should be `Size::Medium` (use `Sizable` trait)
- Support `xs`, `sm`, `md`, `lg` sizes via `Sizable` trait

### Styling and Layout
- Use `StyledExt` trait for CSS-like styling
- Use `h_flex!()` and `v_flex!()` macros for layouts
- Mouse cursor: use `default` for buttons, not `pointer` (desktop convention)
- Use `px()`, `rem()` for dimensions with explicit unit functions
- Theme access via `ActiveTheme` trait: `cx.theme()`

### Error Handling
- Use `anyhow::Result<T>` for application-level errors
- Use `Option<T>` for nullable values
- Panic only for truly unrecoverable errors
- Return `Result<T, E>` from fallible functions

### GPUI Specific Patterns
- **Critical**: Call `gpui_component::init(cx)` at application entry point
- First view in every window must be a `Root` instance
- Use `cx.spawn()` for async operations
- Use `ElementId` for interactive elements
- Use `SharedString` for string data in GPUI context

### Testing Guidelines
- Every component should have a `test_*_builder` test covering the builder pattern
- Focus on complex logic, conditional branching, and edge cases
- Use `#[test]` attribute for unit tests
- Use GPUI's test support features when testing UI components

### Documentation Comments
- Use `///` for public API documentation
- Use `//` for implementation comments sparingly
- Include examples in doc comments for complex components
- Document trait methods and associated types

### Performance Considerations
- Profile rendering changes with `MTL_HUD_ENABLED=1`
- Use `samply` for detailed performance analysis
- Optimize for 60 FPS on standard monitors
- Consider performance impact of component re-renders

## Project Structure

This is a Rust workspace with main crates:
- `crates/ui` - Core UI component library (published as `gpui-component`)
- `crates/story` - Gallery application for components
- `crates/macros` - Procedural macros
- `crates/assets` - Static assets
- `crates/webview` - WebView component support
- `examples/` - Example applications

## Internationalization
- Use `rust-i18n` crate for localization
- Localization files in `crates/ui/locales/`
- Default languages: `en`, `zh-CN`, `zh-HK`

## Platform Support
- macOS (aarch64, x86_64)
- Linux (x86_64) 
- Windows (x86_64)

Always ensure code compiles and passes linting before committing.