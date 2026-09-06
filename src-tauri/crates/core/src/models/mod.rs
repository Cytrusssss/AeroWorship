pub mod monitor;
pub mod scripture;
pub mod slide;
pub mod template;
mod text;

pub use monitor::{flag_primary, monitor_id, select_output_monitor, Monitor};
pub use scripture::{
    parse_reference, parse_scripture_ref, resolve_reference, BookIndex, BookMatch, ParsedReference,
    ScriptureRef,
};
pub use slide::{split_slides, SlideSplit};
pub use template::{
    parse_template, validate_template, AspectRatio, BackgroundFill, Canvas, GradientStop,
    HorizontalAlign, ImageFit, Layer, OutlineEffect, Rect, ShadowEffect, ShapeFill, ShapeGeometry,
    Stroke, TemplateDocument, TemplateError, TemplateMetadata, TextEffects, TextRole,
    TextTransform, Typography, VerticalAlign,
};
