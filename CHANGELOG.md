# Changelog

<!-- next-header -->

## [Unreleased](https://github.com/mdboom/jsonschema-valid/compare/v0.5.1...master) - ReleaseDate

* Updated textwrap dependency

## [0.5.1](https://github.com/mdboom/jsonschema-valid/compare/v0.5.0...v0.5.1) - 2022-11-17

* Disable default features for some dependencies

## [0.5.0](https://github.com/mdboom/jsonschema-valid/compare/v0.4.0...v0.5.0) - 2022-07-11

* Updated dependencies

## [0.4.0](https://github.com/mdboom/jsonschema-valid/compare/v0.3.0...v0.4.0) - 2020-03-12

* Updated to comply with the latest version of the JSON Schema Test Suite.
* **BREAKING CHANGE**: Draft versions are now an enum instead of a trait ([#13](https://github.com/mdboom/jsonschema-valid/pull/13))

### Breaking changes

#### Draft versions are now an enum instead of a trait ([#13](https://github.com/mdboom/jsonschema-valid/pull/13))

The API was changed to not require a trait for the draft version and instead use an enumeration of implemented draft versions.
This simplifies usage slightly.

Old:

```rust
let data: Value = serde_json::from_str(your_json_data)?;
let cfg = jsonschema_valid::Config::from_schema(&schema, Some(&schemas::Draft6))?;
```

New:

```rust
let data: Value = serde_json::from_str(your_json_data)?;
let cfg = jsonschema_valid::Config::from_schema(&schema, Some(schemas::Draft::Draft6))?;
```

## [v0.3.0](https://github.com/mdboom/jsonschema-valid/compare/0.2.0...v0.3.0) (2019-02-26)

* **Major breaking API change:** The main API now returns an `Iterator` over
  `ValidationError` objects, rather than using a callback to report errors.
