# Unreleased

* **Major breaking API change:** The main API now returns an `Iterator` over
  `ValidationError` objects, rather than using a callback to report errors.
