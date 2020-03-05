# jsonschema-valid

A simple JSON schema validator for Rust. Unlike many of the alternatives, this
just focusses on validating a document against a schema and providing nice error
messages. There is no object mapping magic or anything like that.

Supports JSON Schema Drafts 4, 6, and 7.

This repository includes copies of the JSON schema metaschemas, which are
compiled into the binary. These are all listed in the [JSON schema specification
links page](http://json-schema.org/specification-links.html). Specifically:

- `src/draft4.json` comes from `https://json-schema.org/draft-04/schema`
- `src/draft6.json` comes from `https://json-schema.org/draft-06/schema`
- `src/draft7.json` comes from `https://json-schema.org/draft-07/schema`
