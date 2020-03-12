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

## Release process

This project uses [`cargo-release`](https://github.com/sunng87/cargo-release) and follows the [Semantic Versioning](https://semver.org/) process.

To release a new version:

1. Make sure all changes are in the [CHANGELOG.md](CHANGELOG.md). Add missing changes and commit them.
2. Run `cargo release [level]`
    * `[level]` should be one of `major`, `minor` or `patch` depending on the inluded changes.
3. You're done.

## License

This code is released under the Mozilla Public License, v. 2.0.
See [LICENSE](LICENSE).
