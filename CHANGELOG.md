# Unreleased

* Updated to comply with the latest version of the JSON Schema Test Suite.

# v0.3.0 (2019-02-26)

* **Major breaking API change:** The main API now returns an `Iterator` over
  `ValidationError` objects, rather than using a callback to report errors.
