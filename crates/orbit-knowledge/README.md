# orbit-knowledge

`orbit-knowledge` contains Orbit's Rust-side knowledge graph logic: selector parsing,
knowledge pack resolution, working-graph mutation, and lightweight source extraction.
The built-in tree-sitter extractors currently cover Rust, Python, Go, Java,
JavaScript, TypeScript, and TSX source files. TypeScript/TSX coverage was added
in [T20260505-11].
