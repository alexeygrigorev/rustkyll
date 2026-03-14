# Issue 47: Remove Site-Specific Hardcoding

## Problem

The codebase may contain logic or values that are specific to datatalksclub.github.io or other test sites rather than being generic Jekyll-compatible behavior. These need to be identified and made generalizable so rustkyll works as a proper generic Jekyll replacement.

## Requirements

- Audit all source files for any hardcoded references to specific sites (datatalksclub, alexeygrigorev, etc.)
- Audit for any logic that assumes a specific site structure rather than following Jekyll conventions
- Check for hardcoded paths, URLs, domain names, layout names, collection names, or config values that should be driven by the site's own _config.yml
- Make all such cases configurable or driven by standard Jekyll conventions
- Document any findings and changes made

## Dependencies

None.
