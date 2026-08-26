//! Serde types that become the generated TypeScript contract (NFR-33).
//!
//! Template geometry is stored normalised to 0–1, never in pixels (FR-406).

pub mod monitor;
pub mod scripture;
pub mod template;
// Not `pub`: `text` holds no serde type and contributes nothing to the
// TypeScript contract. It exists so `scripture` and `template` share one
// Unicode table instead of keeping two in step.
mod text;

pub use monitor::{flag_primary, monitor_id, select_output_monitor, Monitor};
pub use scripture::{
    parse_reference, parse_scripture_ref, resolve_reference, BookIndex, BookMatch, ParsedReference,
    ScriptureRef,
};
pub use template::{
    parse_template, validate_template, AspectRatio, BackgroundFill, Canvas, GradientStop,
    HorizontalAlign, ImageFit, Layer, OutlineEffect, Rect, ShadowEffect, ShapeFill, ShapeGeometry,
    Stroke, TemplateDocument, TemplateError, TemplateMetadata, TextEffects, TextRole,
    TextTransform, Typography, VerticalAlign,
};
// `SCHEMA_VERSION`, `MAX_DOCUMENT_BYTES` and `MAX_LAYERS` are deliberately not
// re-exported flat. Appendix C's `.aero` format is versioned and size-limited
// too, so all three names will exist twice in this module tree; qualifying them
// as `template::SCHEMA_VERSION` is what keeps the second one from having to be
// spelled differently to avoid a collision it did not cause.
