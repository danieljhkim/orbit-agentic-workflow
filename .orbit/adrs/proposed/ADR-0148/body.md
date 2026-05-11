## Context
Current task artifacts are `path + UTF-8 content`. That is enough for planning duel Markdown or JSON, but it excludes screenshots, binary logs, trace bundles, and generated media. It also lacks checksums and media-type metadata.

## Decision
Store artifacts under `artifacts/files/` and track them with `artifacts/manifest.yaml`. Each manifest entry records logical path, blob path, media type, checksum, size, and attribution.

## Consequences
- Tasks can carry screenshots, binary traces, and structured generated outputs without abusing text fields.
- Artifact integrity can be checked independently of the task envelope.
- CLI display can choose text rendering, summaries, or file paths based on media type.
- Cost: artifact write and read code becomes more complex, and storage now needs size limits, redaction checks, and checksum validation.