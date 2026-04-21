# @stayhydated/astro-wasm-site

Shared Astro site primitives for repositories that publish wasm examples alongside project documentation.

This package is for workspace maintainers who want one place to define:

- shared wasm manifest loading,
- common Astro layouts and components,
- shared site chrome such as navigation,
- generic remark helpers for README-to-site rendering.

Most application and crate users should use the consuming site, such as `web/`, rather than depending on this package directly.
