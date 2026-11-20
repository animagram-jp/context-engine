# Security Assessment

The most likely and the most impactful security problems of this software, and how each is handled.
Review this document before each release, together with the [quality checklist](https://github.com/animagram-jp/.github/blob/main/quality_checklist/README.md).

## Scope

| Item | Content |
|-|-|
| Library | The crate `context-engine` (`src/`). `no_std`, no `unsafe`. It reaches a store only through the `Store` implementations of the application. |
| Precompile | Feature `precompile`: reads the YAML DSL with `serde_yaml_ng` and writes Rust source. It runs at build time. |
| Pipeline | `.github/workflows/`. Publishing to crates.io authenticates through OIDC. |

## Actor

| Actor | Trust | Action |
|-|-|-|
| Application developer | Trusted. | Writes the YAML DSL and the `Store` implementations. |
| Application at runtime | Trusted for the keys, untrusted for the values. | Calls `Context::{get, set, delete, exists}` with a path within a request scope. |
| Request sender of the application | Untrusted. | Produces values that the application sets into the context and that placeholders carry into store keys and arguments. |
| Store | Trusted. | Returns and keeps values, including wire bytes of `Tree`. |
| Pipeline | Trusted. | Tests and publishes the crate. |

## Problem

| Problem | Likelihood | Impact | Handling |
|-|-|-|-|
| A value from a request reaches a placeholder and selects the key of another user or tenant in a store. | Medium | High | Only the paths declared in the DSL are reachable, and an undeclared path is not found. The engine does not validate the values that placeholders carry, so an application sets a session or tenant value only after authenticating it, and never passes a request string as a path. |
| `Tree::unwire` receives crafted bytes. The element count is read from the bytes and reserved before the elements are read, and nesting is not bounded, so a few bytes can abort the process by allocation or stack exhaustion. | Medium | Medium | Malformed lengths return `None`. The count and the depth are not capped, so `unwire` is applied only to bytes that a trusted store returns. |
| Placeholders that refer to each other recurse without end. | Low | Medium | `Context` guards the recursion and returns `ContextError`. |
| Feature `logging` writes paths and arguments, which can hold session identifiers, to the debug log. | Medium | Medium | The feature is off by default. An application that enables it treats the debug log as sensitive. |
| A crafted YAML file exhausts memory in the precompile step. | Low | Low | The DSL is written by the developer and read at build time. Identifier spaces are bounded (`MAX_PATH_ID`, `MAX_WORD_ID`, `MAX_STORE_ID`) and an excess is reported as an error. |
| A vulnerable or malicious dependency, or a leaked publishing credential, alters the published crate. | Low | High | Runtime use has no mandatory dependency. `log` and `serde_yaml_ng` are optional. No long-lived crates.io token exists, because publishing uses trusted publishing from the `crates-io` environment on a version tag. Actions are pinned by commit SHA. |
