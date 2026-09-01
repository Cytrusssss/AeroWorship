//! The template document: its types, its parser and its validator
//! (FR-401, PRD §6.7, Appendix B).
//!
//! Appendix B *is* the specification. Nothing here is invented shape: every
//! field, every enum spelling and every nesting level below is transcribed from
//! it, and the places Appendix B leaves a value unconstrained are called out by
//! name in the comments on the constants that constrain them.
//!
//! **Why these field names are `snake_case` when `Monitor`'s are `camelCase`.**
//! A template is a *document*, not a wire record. Appendix B spells its keys
//! `schema_version`, `align_h`, `blur_px_ratio`; Appendix C spells the `.aero`
//! session file the same way; Appendix D's `Monitor` and `Slide` are camelCase
//! because they are IPC payloads. The document form is the one that reaches a
//! disk, an `.aerotpl` export on a stranger's machine (FR-409) and the
//! `templates.document` column (Appendix A) — a rename here would make every
//! file already written unreadable and would put this crate at odds with the
//! appendix it claims to implement. So there is no `rename_all` on any struct
//! in this module, and the generated TypeScript is snake_case for the same
//! reason.
//!
//! **Untrusted input, and what that costs (NFR-28).** A template arrives from a
//! file this build did not write, and whatever survives validation is handed to
//! a renderer running in the *same web origin* as the Control Panel (ADR-0018).
//! So the stance is **reject, never repair**:
//!
//! * Every struct and every tagged enum carries `#[serde(deny_unknown_fields)]`.
//!   Serde ignores unknown fields by default, which would let
//!   `{"type":"text", …, "onLoad":"…"}` load silently and sit in the document
//!   until some later renderer spread it into a DOM node. The price is real and
//!   is accepted on purpose: **adding a field to Appendix B becomes a breaking
//!   change**, because an older build will refuse the newer document outright.
//!   That is what `schema_version` is for, and refusing loudly is the behaviour
//!   this module wants at that boundary anyway.
//! * The layer `type` and the fill/geometry `kind` are internally-tagged enums,
//!   so an unrecognised tag is an error and not a variant. `{"type":"script"}`
//!   fails with ``unknown variant `script` `` — FR-401's acceptance case — and
//!   it fails for the general reason rather than by a rule about the word
//!   "script", so `{"type":"iframe"}` and `{"type":"webview"}` fail with it.
//! * Nothing is clamped. A coordinate of `9.0` is refused, not pulled back to
//!   `1.5`; a repaired template is a design nobody authored, put on a projector
//!   in the middle of a service. This is the lesson FR-205 already paid for
//!   with backwards verse ranges.
//! * Colours, ids, font names and SVG path data are matched against
//!   **allowlists**, not scanned for bad substrings. A colour is `#RGB` or
//!   `#RRGGBB` and nothing else, so `url(…)`, `expression(…)` and a stray `;`
//!   closing an inline style are unrepresentable rather than filtered out.
//!
//! **What this module deliberately does not check**, because it cannot and stay
//! pure (PRD §6.1). Each of these has an owner, and none of them is here:
//!
//! | Not checked | Owner |
//! | --- | --- |
//! | That a `media_id` names an asset that exists, and that it resolves inside a permitted media root | NFR-15 / FR-409, at the caller, before any image is loaded |
//! | That an SVG `d` value is a well-formed path — only its character set is checked here | FR-403's path-grammar allowlist |
//! | That `font_family` is installed on this machine | FR-402, and the substitution risk in PRD §7 |
//! | The document's byte size, when the caller deserialised it itself instead of calling [`parse_template`] | the caller — see [`validate_template`] |
//! | That `name` and `author` are safe to place in markup — both are human text and may legitimately hold `<`, `>`, `&`, `"` and `'` | the consumer, which must put them in a text node and never assemble them into HTML |
//!
//! Doc comments on the public types are copied verbatim into
//! `src/shared/bindings/*.ts` by ts-rs, so they are written for a frontend
//! reader; this module's own reasoning is in ordinary `//` comments, which are
//! not copied.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::text;

// ---------------------------------------------------------------------------
// Limits
//
// The first three are Appendix B's own, from "Validation rules enforced on
// load". Everything below them is a bound Appendix B does not state; each
// carries the reasoning for its number, because a limit with no reason behind
// it is a limit the next reader quietly raises.
// ---------------------------------------------------------------------------

/// The only template schema version this build understands (Appendix B).
pub const SCHEMA_VERSION: u32 = 1;

/// Largest template document [`parse_template`] will look at, in bytes
/// (Appendix B: "document under 256 KB").
pub const MAX_DOCUMENT_BYTES: usize = 256 * 1024;

/// Most layers a template may hold (Appendix B: "at most 32 layers").
pub const MAX_LAYERS: usize = 32;

/// Span every normalised **position** must fall inside (Appendix B: "all
/// normalized coordinates within `[-0.5, 1.5]` (bleed allowed, absurd values
/// rejected)").
const POSITION: Bounds = Bounds {
    min: -0.5,
    max: 1.5,
};

// An **extent** — a width or a height — is held to `[0, 1.5]` rather than to
// `POSITION`. Appendix B says "coordinates", and a negative width is not a
// coordinate that bled off the canvas; it is a malformed box. It would reach an
// SVG `width` attribute or a CSS `width`, where it is an error in one and
// invalid in the other, and the two do not agree about what happens next. Zero
// stays legal: a zero-width layer is invisible, which is a design choice an
// author can undo, not a hazard.
const EXTENT: Bounds = Bounds { min: 0.0, max: 1.5 };

/// Span for a fraction of one: an opacity, a gradient stop offset.
const UNIT: Bounds = Bounds { min: 0.0, max: 1.0 };

// A normalised length that is not a coordinate: a blur radius, a stroke width,
// a corner radius, a type size. Appendix B gives these no range at all, so
// `[0, 1]` is chosen as the widest span that still means something — each is a
// fraction of the canvas, and a blur or a corner radius larger than the whole
// canvas describes nothing. Unlike `EXTENT` there is no bleed case for a
// radius, so the ceiling is 1.0 rather than 1.5.
const LENGTH: Bounds = Bounds { min: 0.0, max: 1.0 };

// Letter spacing is the one normalised length that may be negative: tightening
// type is ordinary typography and `-0.01` of canvas height is a normal value.
// Bounded symmetrically for that reason, and bounded at all because a spacing
// of 1e6 pushes a single glyph a million canvas heights sideways, where the
// browser still has to lay it out.
const LETTER_SPACING: Bounds = Bounds {
    min: -1.0,
    max: 1.0,
};

// A multiple of the type size rather than a fraction of the canvas, so it has a
// span of its own. Below 1.0 is tight leading and legitimate; 0.0 stacks every
// line on one baseline and a negative runs the text upwards. The ceiling is
// generous because a caption block with airy leading is a real design.
const LINE_HEIGHT: Bounds = Bounds {
    min: 0.1,
    max: 10.0,
};

// A full turn either way. CSS would take any angle and wrap it; refusing beyond
// one turn costs an author nothing and keeps the number readable in the Builder
// (FR-407), where "1080°" is a sign the file was generated rather than
// authored.
const GRADIENT_ANGLE: Bounds = Bounds {
    min: -360.0,
    max: 360.0,
};

/// The CSS `font-weight` range for variable fonts — the widest thing a renderer
/// can pass through. Appendix B shows 700 and 400 and gives no bound.
const WEIGHT: IntBounds = IntBounds { min: 1, max: 1000 };

// `max_lines` "drives slide splitting" (Appendix B), which makes it a loop
// bound inside FR-310 rather than a decoration — and an unbounded loop bound
// taken from a file this build did not write is a denial of service with extra
// steps. 64 is the ceiling because it cannot be reached honestly: at the
// smallest normalised size the worked example uses (0.016 of canvas height),
// 64 single-spaced lines is already more than a canvas holds. Anything above it
// is not a design.
const MAX_LINES: IntBounds = IntBounds { min: 1, max: 64 };

// The authoring reference is "not a constraint" (Appendix B), so it is
// deliberately *not* cross-checked against `aspect_ratio` — the appendix says
// so in as many words. It is still bounded: it is a divisor and a multiplier in
// the Builder, zero would divide by zero, and 16384 is the largest dimension
// commodity GPUs will allocate a texture for, so a canvas beyond it cannot be
// painted on any machine this ships to.
const REFERENCE_DIMENSION: IntBounds = IntBounds {
    min: 1,
    max: 16_384,
};

// A gradient needs two ends to be a gradient: one stop is a solid fill spelled
// the long way and zero is nothing at all. The ceiling sits well above any
// gradient a person authors by hand and well below a number that makes a
// renderer work. This is the only unbounded array Appendix B leaves *inside* a
// layer, so `MAX_LAYERS` does not cover it.
const GRADIENT_STOPS: IntBounds = IntBounds { min: 2, max: 16 };

/// Most families a `font_fallback` stack may name. A stack ends at a generic
/// family (`sans-serif`); eight steps is more than any needs to get there.
const FONT_FALLBACKS: IntBounds = IntBounds { min: 0, max: 8 };

/// Longest a single font family name may be, in characters.
const MAX_FONT_FAMILY_CHARS: usize = 64;

/// Longest the template name, and the metadata author, may be, in characters.
//
// Both are shown in a picker (FR-408) beside other templates. A name of 100_000
// characters is not a name; it is a layout attack on the list it appears in,
// and the list cannot defend itself once the value is in the database.
const MAX_DISPLAY_CHARS: usize = 120;

/// Longest an SVG `d` value may be, in characters.
//
// FR-403 owns the path *grammar*; this module owns the fact that the string is
// bounded at all, because FR-403's parser has to be handed something finite. A
// hand-drawn decorative frame is a few hundred characters; a traced photograph
// is hundreds of thousands, and is not what a scrim layer is for.
const MAX_PATH_DATA_CHARS: usize = 4096;

/// Longest a `viewbox` string may be, in characters.
const MAX_VIEWBOX_CHARS: usize = 64;

/// Span an ISO-8601 timestamp's length must fall in, in characters.
const MIN_TIMESTAMP_CHARS: usize = 20;
/// See [`MIN_TIMESTAMP_CHARS`]. Long enough for fractional seconds and an
/// offset.
const MAX_TIMESTAMP_CHARS: usize = 32;

// ---------------------------------------------------------------------------
// The document (Appendix B §B.1)
// ---------------------------------------------------------------------------

/// A complete template: a canvas, an ordered stack of layers, a safe area and
/// its metadata.
///
/// This is the document stored in `templates.document` and exported as
/// `.aerotpl` (FR-409). The field names are exactly Appendix B's, so a value of
/// this type and the JSON on disk are one thing spelled twice.
///
/// **A value of this type has not necessarily been validated**: deserialising
/// one checks its *shape* and never its values, so ranges, colour spelling and
/// id spelling hold only for a document that came back from `parse_template` or
/// `validate_template`.
//
// **`Deserialize` is derived, and that is a loaded gun pointed at the caller.**
// A `TemplateDocument` can be produced without ever passing through
// `parse_template` — by `serde_json::from_value`, or by a future
// `save_template` command taking one straight off the IPC boundary. What serde
// alone guarantees is the *shape*: the right fields, no unknown ones, the right
// JSON types. It guarantees nothing about ranges, counts, colour spelling, id
// spelling or path data. All of that is `validate_template`, and a caller that
// skips it holds a document that satisfies this type and violates Appendix B.
// The same trap `ScriptureRef` documents for its own invariants, one layer up.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct TemplateDocument {
    /// Version of the template schema this document is written in. Only `1`
    /// exists; anything else is refused on load with a message naming the
    /// version, rather than as a parse failure.
    pub schema_version: u32,
    /// Stable identity of the template, as a canonical UUID.
    pub id: String,
    /// What the operator sees in the template picker. Never blank.
    pub name: String,
    /// The abstract box every coordinate in this document is relative to.
    pub canvas: Canvas,
    /// Painted in array order: index 0 is the back layer and the last entry is
    /// on top. At most 32 entries.
    pub layers: Vec<Layer>,
    /// The region content is expected to stay inside, drawn as a guide in the
    /// Builder (FR-407). It constrains nothing at render time.
    pub safe_area: Rect,
    /// Authoring provenance. Not used for rendering.
    pub metadata: TemplateMetadata,
}

/// The canvas a template is authored against.
///
/// Only `aspect_ratio` affects rendering. The reference size is authoring
/// information — geometry is normalised (FR-406), so the same document paints
/// proportionally on any output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct Canvas {
    /// Shape of the canvas box.
    pub aspect_ratio: AspectRatio,
    /// Width the template was authored against, in pixels. A reference only: it
    /// is not checked against `aspect_ratio` and it constrains no output.
    pub reference_width: u32,
    /// Height the template was authored against, in pixels. Same status as
    /// `reference_width`.
    pub reference_height: u32,
}

/// The canvas shapes a template may be authored for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum AspectRatio {
    /// Widescreen: the shape of nearly every projector and TV panel.
    #[serde(rename = "16:9")]
    SixteenNine,
    /// The shape of many older business projectors.
    #[serde(rename = "16:10")]
    SixteenTen,
    /// The shape of a legacy 4:3 projector.
    #[serde(rename = "4:3")]
    FourThree,
}

/// One painted layer.
///
/// The `type` field selects the variant. A layer whose `type` is anything other
/// than `background`, `shape` or `text` is rejected when the document is
/// loaded: there is no pass-through case, and no way to carry a payload the
/// renderer does not understand.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum Layer {
    /// Fills the whole canvas: a colour, a gradient or an image.
    Background {
        /// Canonical UUID, unique within this document.
        id: String,
        /// `false` hides the layer without removing it.
        visible: bool,
        /// What the background is painted with.
        fill: BackgroundFill,
    },
    /// A scrim, a frame, or a piece of decorative geometry.
    Shape {
        /// Canonical UUID, unique within this document.
        id: String,
        /// `false` hides the layer without removing it.
        visible: bool,
        /// Where and what the shape is.
        geometry: ShapeGeometry,
        /// Interior colour.
        fill: ShapeFill,
        /// Outline. A `width` of 0 draws none.
        stroke: Stroke,
        /// Blur applied to the layer, as a fraction of canvas height. 0 is
        /// sharp.
        blur_px_ratio: f64,
    },
    /// A slot bound to a content role. It holds no literal text: the session
    /// item supplies the text for its role at render time (PRD §6.7), and a
    /// role the content type does not provide renders nothing.
    Text {
        /// Canonical UUID, unique within this document.
        id: String,
        /// `false` hides the layer without removing it.
        visible: bool,
        /// Which piece of the item's content lands in this slot.
        role: TextRole,
        /// The slot, in normalised units.
        #[serde(rename = "box")]
        text_box: Rect,
        /// How the text in the slot is set.
        typography: Typography,
        /// Shadow and outline treatment, for legibility over a photograph.
        effects: TextEffects,
    },
}

impl Layer {
    /// The layer's id, whatever kind of layer it is.
    pub fn id(&self) -> &str {
        match self {
            Self::Background { id, .. } | Self::Shape { id, .. } | Self::Text { id, .. } => id,
        }
    }

    /// Whether the layer is painted.
    pub fn is_visible(&self) -> bool {
        match self {
            Self::Background { visible, .. }
            | Self::Shape { visible, .. }
            | Self::Text { visible, .. } => *visible,
        }
    }

    /// The media asset this layer references, if it references one.
    ///
    /// The only place a template can point outside itself, so it is the list a
    /// caller walks to populate `template_media` (Appendix A) and to check every
    /// reference against the permitted media roots (NFR-15) — a check this
    /// crate cannot make, because it involves the filesystem.
    pub fn media_id(&self) -> Option<&str> {
        match self {
            Self::Background {
                fill: BackgroundFill::Image { media_id, .. },
                ..
            } => Some(media_id),
            _ => None,
        }
    }
}

/// What a background layer is painted with. The `kind` field selects the
/// variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum BackgroundFill {
    /// One flat colour across the canvas.
    Solid {
        /// `#RGB` or `#RRGGBB`.
        color: String,
    },
    /// A linear gradient.
    Gradient {
        /// Direction in degrees, as CSS measures it: 0 points up, 180 points
        /// down.
        angle_deg: f64,
        /// Between 2 and 16 colour stops, in the order given.
        stops: Vec<GradientStop>,
    },
    /// A registered media asset, painted to fit.
    Image {
        /// Canonical UUID of the asset in `media_assets`. That it exists, and
        /// that it resolves inside a permitted media root, is checked by the
        /// caller and not by the template validator.
        media_id: String,
        /// How the image is fitted to the canvas.
        fit: ImageFit,
        /// 0 is fully transparent, 1 fully opaque.
        opacity: f64,
        /// Blur applied to the image, as a fraction of canvas height. 0 is
        /// sharp.
        blur_px_ratio: f64,
    },
}

/// One stop of a gradient.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct GradientStop {
    /// Position along the gradient: 0 at the start, 1 at the end.
    pub offset: f64,
    /// `#RGB` or `#RRGGBB`.
    pub color: String,
}

/// How a background image is fitted to a canvas of a different shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum ImageFit {
    /// Fill the canvas, cropping the overflow. Aspect ratio preserved.
    Cover,
    /// Fit inside the canvas, leaving bars. Aspect ratio preserved.
    Contain,
    /// Fill the canvas exactly. Aspect ratio not preserved.
    Stretch,
    /// Repeat the image at its own size.
    Tile,
}

/// Where and what a shape layer is. The `kind` field selects the variant, and
/// every variant carries the same normalised box.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum ShapeGeometry {
    /// A rectangle.
    Rect {
        /// Left edge, normalised.
        x: f64,
        /// Top edge, normalised.
        y: f64,
        /// Width, normalised.
        w: f64,
        /// Height, normalised.
        h: f64,
    },
    /// A rectangle with rounded corners.
    RoundedRect {
        /// Left edge, normalised.
        x: f64,
        /// Top edge, normalised.
        y: f64,
        /// Width, normalised.
        w: f64,
        /// Height, normalised.
        h: f64,
        /// Corner radius as a fraction of canvas height. Only this kind of
        /// shape has one.
        corner_radius: f64,
    },
    /// An ellipse inscribed in the box.
    Ellipse {
        /// Left edge, normalised.
        x: f64,
        /// Top edge, normalised.
        y: f64,
        /// Width, normalised.
        w: f64,
        /// Height, normalised.
        h: f64,
    },
    /// An arbitrary SVG path, drawn inside the box.
    Path {
        /// Left edge, normalised.
        x: f64,
        /// Top edge, normalised.
        y: f64,
        /// Width, normalised.
        w: f64,
        /// Height, normalised.
        h: f64,
        /// SVG path data. Only path commands, numbers and separators may appear
        /// in it; that the commands form a legal path is checked separately
        /// before it is drawn (FR-403).
        d: String,
        /// The `viewBox` the path data is expressed in: four numbers, usually
        /// `0 0 1 1`.
        viewbox: String,
    },
}

/// The interior colour of a shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct ShapeFill {
    /// `#RGB` or `#RRGGBB`.
    pub color: String,
    /// 0 is fully transparent, 1 fully opaque. A scrim is typically 0.5–0.7.
    pub opacity: f64,
}

/// The outline of a shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct Stroke {
    /// `#RGB` or `#RRGGBB`.
    pub color: String,
    /// Line width as a fraction of canvas height. 0 draws no outline.
    pub width: f64,
    /// 0 is fully transparent, 1 fully opaque.
    pub opacity: f64,
}

/// The semantic slot a text layer binds to (PRD §6.7).
///
/// A template declares the role; the session item supplies the text. A role the
/// current content type does not provide simply renders nothing, which is what
/// makes one template reusable across songs, scripture and media.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum TextRole {
    /// Lyric lines, or verse text.
    Primary,
    /// Optional preview of the next section. Songs only.
    Secondary,
    /// Section label such as "Chorus", or a passage reference such as
    /// `Yohanes 3:16 (TB)`.
    Reference,
    /// Copyright line, CCLI number, version copyright, or the source deck name.
    Attribution,
}

/// A normalised box: the text slot of a layer, or the safe area of a document.
///
/// `x` and `y` are the top-left corner. All four numbers are fractions of the
/// canvas, never pixels (FR-406). Positions may run slightly outside `0..1` for
/// a bleed; extents may not be negative.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct Rect {
    /// Left edge, normalised.
    pub x: f64,
    /// Top edge, normalised.
    pub y: f64,
    /// Width, normalised.
    pub w: f64,
    /// Height, normalised.
    pub h: f64,
}

/// How the text in a slot is set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct Typography {
    /// Preferred font family. Whether it is installed on the machine showing
    /// the slide is not checked here.
    pub font_family: String,
    /// Families to fall back to, in order, ending at a generic family such as
    /// `sans-serif`. At most 8.
    pub font_fallback: Vec<String>,
    /// CSS font weight, 1–1000. 400 is regular, 700 is bold.
    pub weight: u16,
    /// Type size as a fraction of canvas height — this is what makes a template
    /// resolution-independent (FR-406).
    pub size: f64,
    /// The floor the renderer may shrink to before splitting the content across
    /// slides instead (FR-310). Never larger than `size`.
    pub min_size: f64,
    /// Line box height as a multiple of the type size.
    pub line_height: f64,
    /// Extra space between characters, as a fraction of canvas height. May be
    /// negative, to tighten.
    pub letter_spacing: f64,
    /// Case transformation applied at render time.
    pub transform: TextTransform,
    /// `#RGB` or `#RRGGBB`.
    pub color: String,
    /// Horizontal alignment inside the slot.
    pub align_h: HorizontalAlign,
    /// Vertical alignment inside the slot.
    pub align_v: VerticalAlign,
    /// Lines that fit in the slot before the content is split onto another
    /// slide (FR-310). 1–64.
    pub max_lines: u16,
}

/// Case transformation applied to a text slot at render time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum TextTransform {
    /// Render the text as it was written.
    None,
    /// Render every letter as a capital.
    Uppercase,
    /// Render the first letter of every word as a capital.
    Capitalize,
}

/// Horizontal alignment of text inside its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum HorizontalAlign {
    /// Against the left edge of the slot.
    Left,
    /// Centred in the slot.
    Center,
    /// Against the right edge of the slot.
    Right,
}

/// Vertical alignment of text inside its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum VerticalAlign {
    /// Against the top edge of the slot.
    Top,
    /// Centred in the slot.
    Middle,
    /// Against the bottom edge of the slot.
    Bottom,
}

/// Legibility treatment for a text slot over a photographic background.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct TextEffects {
    /// Drop shadow behind the text.
    pub shadow: ShadowEffect,
    /// Outline drawn around the glyphs.
    pub outline: OutlineEffect,
}

/// A drop shadow. Every field is still carried when `enabled` is `false`, so
/// turning it back on in the Builder restores the settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct ShadowEffect {
    /// Whether the shadow is drawn.
    pub enabled: bool,
    /// `#RGB` or `#RRGGBB`.
    pub color: String,
    /// 0 is fully transparent, 1 fully opaque.
    pub opacity: f64,
    /// Blur radius as a fraction of canvas height.
    pub blur: f64,
    /// Horizontal offset as a fraction of canvas height. May be negative.
    pub offset_x: f64,
    /// Vertical offset as a fraction of canvas height. May be negative.
    pub offset_y: f64,
}

/// An outline around the glyphs. As with the shadow, its settings survive being
/// disabled.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct OutlineEffect {
    /// Whether the outline is drawn.
    pub enabled: bool,
    /// `#RGB` or `#RRGGBB`.
    pub color: String,
    /// Outline width as a fraction of canvas height.
    pub width: f64,
}

/// Authoring provenance. None of it affects rendering.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct TemplateMetadata {
    /// When the template was first saved, as an ISO-8601 timestamp.
    ///
    /// Checked for shape and length only — this crate does not own a calendar,
    /// so `2026-13-45T99:99:99Z` is accepted as a string and is not a date.
    pub created_at: String,
    /// When the template was last saved. Same status as `created_at`.
    pub updated_at: String,
    /// Who authored it. May be empty.
    pub author: String,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Why a template was refused.
///
/// Spelled out by hand rather than derived, for the reason `db::DbError` gives:
/// a `thiserror` dependency would buy a `Display` impl at the cost of two more
/// crates in a 15 MB installer (NFR-16).
#[derive(Debug)]
pub enum TemplateError {
    /// The document is larger than [`MAX_DOCUMENT_BYTES`].
    ///
    /// Raised before any parsing, so an enormous file costs one length check
    /// and nothing else.
    TooLarge {
        /// Size of the document handed in, in bytes.
        bytes: usize,
        /// The limit it exceeded.
        limit: usize,
    },

    /// The text is not JSON, or is JSON that is not a template: a missing
    /// field, an unknown field, a wrong type, an unrecognised layer `type`.
    ///
    /// **The message quotes the input.** `serde_json` names the offending field
    /// or variant, which is exactly what makes the diagnostic useful and also
    /// means untrusted text reaches whoever displays it. A caller putting this
    /// on a screen or in a log must escape and truncate it there; this crate
    /// cannot, because an escape correct for a terminal is wrong for HTML. The
    /// same caution `BookIndex::ambiguous_spellings` carries.
    ///
    /// **Truncate to a length you have chosen.** `serde_json` copies an unknown
    /// field or variant name out of the input exactly as it found it, and a
    /// name may be as long as the document, so a message from
    /// [`parse_template`] is bounded only by [`MAX_DOCUMENT_BYTES`] — up to
    /// 256 KB from a single key. A caller that deserialised the document itself
    /// has no bound at all.
    Syntax(serde_json::Error),

    /// `schema_version` is not [`SCHEMA_VERSION`].
    ///
    /// Kept apart from [`TemplateError::Syntax`] on purpose, and detected
    /// before the document is deserialised: a template written by a newer build
    /// would otherwise fail with `unknown field`, which sends the reader
    /// looking for a corrupt file instead of for an upgrade. That is the stance
    /// FR-708 sets for `.aero` files, applied here for the same reason.
    UnsupportedSchemaVersion {
        /// Version recorded in the document.
        found: u64,
        /// The version this build understands.
        supported: u32,
    },

    /// A number is outside the span Appendix B allows for it.
    OutOfRange {
        /// Dotted path to the field, such as `layers[2].typography.size`.
        field: String,
        /// The value found.
        value: f64,
        /// Smallest accepted value.
        min: f64,
        /// Largest accepted value.
        max: f64,
    },

    /// A count, a length, or an integer setting is outside its allowed span.
    IntegerOutOfRange {
        /// Dotted path to the field, such as `layers`.
        field: String,
        /// The value found.
        value: u64,
        /// Smallest accepted value.
        min: u64,
        /// Largest accepted value.
        max: u64,
    },

    /// A string does not match the grammar its field allows.
    ///
    /// The offending text is **not** carried. `reason` is a fixed phrase chosen
    /// inside this crate, so this variant can be displayed anywhere without the
    /// escaping problem [`TemplateError::Syntax`] has.
    Malformed {
        /// Dotted path to the field.
        field: String,
        /// What the field must look like, as a phrase that follows the field
        /// name.
        reason: &'static str,
    },

    /// Two layers share an id.
    ///
    /// Ids key the renderer's element list and the Builder's selection, so a
    /// duplicate makes one of the two layers unaddressable.
    DuplicateLayerId {
        /// Dotted path to the second layer carrying the id.
        field: String,
    },
}

impl fmt::Display for TemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { bytes, limit } => {
                write!(
                    f,
                    "template document is {bytes} bytes; the limit is {limit}"
                )
            }
            Self::Syntax(err) => write!(f, "template is not a valid Appendix B document: {err}"),
            Self::UnsupportedSchemaVersion { found, supported }
                if *found > u64::from(*supported) =>
            {
                write!(
                    f,
                    "this template was created by a newer version of AeroWorship \
                     (schema version {found}; this build understands {supported}). \
                     Update AeroWorship to open it — it has not been modified."
                )
            }
            Self::UnsupportedSchemaVersion { found, supported } => write!(
                f,
                "this template declares schema version {found}, which this build \
                 does not know how to read (it understands {supported})"
            ),
            Self::OutOfRange {
                field,
                value,
                min,
                max,
            } => write!(f, "{field} is {value}; it must be between {min} and {max}"),
            Self::IntegerOutOfRange {
                field,
                value,
                min,
                max,
            } => write!(f, "{field} is {value}; it must be between {min} and {max}"),
            Self::Malformed { field, reason } => write!(f, "{field} {reason}"),
            Self::DuplicateLayerId { field } => write!(
                f,
                "{field} repeats an id an earlier layer already uses; layer ids must be unique"
            ),
        }
    }
}

impl Error for TemplateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Syntax(err) => Some(err),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for TemplateError {
    fn from(err: serde_json::Error) -> Self {
        Self::Syntax(err)
    }
}

// ---------------------------------------------------------------------------
// Loading
// ---------------------------------------------------------------------------

/// Reads a template document from JSON text and validates it (FR-401).
///
/// This is the load path: the whole of Appendix B's "validation rules enforced
/// on load" that can be decided without touching a disk, in order — size,
/// `schema_version`, shape, then every rule [`validate_template`] applies.
///
/// **It takes text, never a path.** Opening the file, and deciding which files
/// may be opened at all, belongs to the caller; this function is pure, so a
/// hostile `.aerotpl` can be put in front of it without a filesystem anywhere
/// (PRD §6.1).
///
/// The first failure is returned, not a list of them. A template that breaks
/// one rule does not load whether it breaks one more or twenty; showing an
/// author everything wrong at once is the Builder's job (FR-407), over a
/// document it is holding rather than over a file.
pub fn parse_template(json: &str) -> Result<TemplateDocument, TemplateError> {
    // Bytes, not characters, and measured before anything reads the text. This
    // is the only limit that can be applied cheaply to a file of any size, and
    // applying it first is what stops a 2 GB `.aerotpl` from becoming a 2 GB
    // allocation inside `serde_json`.
    if json.len() > MAX_DOCUMENT_BYTES {
        return Err(TemplateError::TooLarge {
            bytes: json.len(),
            limit: MAX_DOCUMENT_BYTES,
        });
    }

    // The version is read through a deliberately lenient probe — one field, and
    // unknown fields tolerated — *before* the strict deserialise below. That
    // ordering is the whole point: a version 2 document will not fit this
    // build's types, so the strict pass would report `unknown field` about
    // whichever new field it met first, and the operator would be told their
    // file is corrupt when it is merely newer. A probe that fails outright (no
    // `schema_version` at all, or one that is not a whole number) falls through
    // on purpose, because then the strict pass has the better diagnostic.
    if let Some(found) = probe_schema_version(json) {
        if found != u64::from(SCHEMA_VERSION) {
            return Err(TemplateError::UnsupportedSchemaVersion {
                found,
                supported: SCHEMA_VERSION,
            });
        }
    }

    let document: TemplateDocument = serde_json::from_str(json)?;
    validate_template(&document)?;
    Ok(document)
}

/// Reads `schema_version` out of a document without committing to the rest of
/// it.
fn probe_schema_version(json: &str) -> Option<u64> {
    // Note the absence of `deny_unknown_fields`: this type exists precisely to
    // read one field out of a document it does not otherwise understand.
    #[derive(Deserialize)]
    struct VersionProbe {
        schema_version: u64,
    }

    serde_json::from_str::<VersionProbe>(json)
        .ok()
        .map(|probe| probe.schema_version)
}

/// Checks every Appendix B rule serde cannot express, over a document that is
/// already parsed.
///
/// [`parse_template`] calls this. Call it directly for a document that arrived
/// some other way — from the frontend across the IPC boundary, or built in
/// code. **A `TemplateDocument` that has not been through this function
/// satisfies its Rust type and may still violate Appendix B**: serde checks the
/// shape, this checks the values.
///
/// What it enforces, all of it refusal and none of it correction:
///
/// * `schema_version` is the one this build knows.
/// * At most 32 layers, each with a unique canonical-UUID id.
/// * Every normalised position within `[-0.5, 1.5]` and every extent within
///   `[0, 1.5]`; every opacity, blur, radius and type size within its own span.
/// * `size` above zero, and `min_size` no larger than `size`.
/// * Every colour is `#RGB` or `#RRGGBB` — no CSS functions, no named colours.
/// * Every font family is letters, digits, spaces, hyphens and underscores.
/// * SVG path data holds only path commands, numbers and separators; a
///   `viewbox` is four numbers with a positive width and height.
/// * The template name is not blank, and neither it nor the author carries an
///   invisible character or a path separator. Both may carry markup
///   metacharacters — see the module header for whose problem that is.
///
/// **The limits above are a byte budget as well as a value check.** Every
/// `Vec` in the document has a maximum length and every `String` a maximum
/// character count, so the largest document that can pass this function is
/// about 210 KB — dominated by 32 layers of 4 KB path data — against the 256 KB
/// [`MAX_DOCUMENT_BYTES`] ceiling. The margin is what makes a future field
/// added without a bound of its own visible: it breaks that arithmetic, and
/// the byte ceiling is where it shows up first.
///
/// **Two limits it does not apply, and the caller must.** It does not know how
/// many bytes the document occupied, so [`MAX_DOCUMENT_BYTES`] is not enforced
/// here: a caller that deserialised the document itself has to cap the input
/// before deserialising, exactly as [`parse_template`] does. And it does not
/// bound *nesting*, because it does not have to — this document type is not
/// recursive, so its depth is fixed by the schema and no file can grow it. A
/// caller feeding `serde_json` directly still gets that crate's own recursion
/// limit, which stops arbitrarily nested JSON before it reaches these types at
/// all.
pub fn validate_template(document: &TemplateDocument) -> Result<(), TemplateError> {
    if u64::from(document.schema_version) != u64::from(SCHEMA_VERSION) {
        return Err(TemplateError::UnsupportedSchemaVersion {
            found: u64::from(document.schema_version),
            supported: SCHEMA_VERSION,
        });
    }

    check_uuid("id", &document.id)?;
    check_display_text("name", &document.name, false)?;
    check_canvas(&document.canvas)?;
    check_rect("safe_area", &document.safe_area)?;
    check_metadata(&document.metadata)?;

    check_int("layers", count(document.layers.len()), &LAYER_COUNT)?;
    // Compared lowercased because a UUID's letters are not significant, so
    // `…A01` and `…a01` are one id spelled twice; both would otherwise be
    // accepted here and then collide inside whichever consumer normalises
    // first.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (index, layer) in document.layers.iter().enumerate() {
        let field = format!("layers[{index}]");
        check_layer(&field, layer)?;
        if !seen.insert(layer.id().to_ascii_lowercase()) {
            return Err(TemplateError::DuplicateLayerId { field });
        }
    }

    Ok(())
}

/// Appendix B's layer cap, as an integer bound, so it is checked by the same
/// helper as every other count.
const LAYER_COUNT: IntBounds = IntBounds {
    min: 0,
    max: MAX_LAYERS as u64,
};

fn check_canvas(canvas: &Canvas) -> Result<(), TemplateError> {
    check_int(
        "canvas.reference_width",
        u64::from(canvas.reference_width),
        &REFERENCE_DIMENSION,
    )?;
    check_int(
        "canvas.reference_height",
        u64::from(canvas.reference_height),
        &REFERENCE_DIMENSION,
    )
}

fn check_metadata(metadata: &TemplateMetadata) -> Result<(), TemplateError> {
    check_timestamp("metadata.created_at", &metadata.created_at)?;
    check_timestamp("metadata.updated_at", &metadata.updated_at)?;
    // Blank is allowed here and refused for `name`: an unattributed template is
    // ordinary, an unnamed one is a blank row in the picker — the argument
    // `models::monitor::reported_name` makes about a display with no name.
    check_display_text("metadata.author", &metadata.author, true)
}

fn check_layer(field: &str, layer: &Layer) -> Result<(), TemplateError> {
    check_uuid(&join(field, "id"), layer.id())?;

    match layer {
        Layer::Background { fill, .. } => check_background_fill(&join(field, "fill"), fill),
        Layer::Shape {
            geometry,
            fill,
            stroke,
            blur_px_ratio,
            ..
        } => {
            check_geometry(&join(field, "geometry"), geometry)?;
            check_colour(&join(field, "fill.color"), &fill.color)?;
            check_number(&join(field, "fill.opacity"), fill.opacity, &UNIT)?;
            check_colour(&join(field, "stroke.color"), &stroke.color)?;
            check_number(&join(field, "stroke.width"), stroke.width, &LENGTH)?;
            check_number(&join(field, "stroke.opacity"), stroke.opacity, &UNIT)?;
            check_number(&join(field, "blur_px_ratio"), *blur_px_ratio, &LENGTH)
        }
        Layer::Text {
            text_box,
            typography,
            effects,
            ..
        } => {
            check_rect(&join(field, "box"), text_box)?;
            check_typography(&join(field, "typography"), typography)?;
            check_effects(&join(field, "effects"), effects)
        }
    }
}

fn check_background_fill(field: &str, fill: &BackgroundFill) -> Result<(), TemplateError> {
    match fill {
        BackgroundFill::Solid { color } => check_colour(&join(field, "color"), color),
        BackgroundFill::Gradient { angle_deg, stops } => {
            check_number(&join(field, "angle_deg"), *angle_deg, &GRADIENT_ANGLE)?;
            check_int(&join(field, "stops"), count(stops.len()), &GRADIENT_STOPS)?;
            for (index, stop) in stops.iter().enumerate() {
                let stop_field = format!("{field}.stops[{index}]");
                check_number(&join(&stop_field, "offset"), stop.offset, &UNIT)?;
                check_colour(&join(&stop_field, "color"), &stop.color)?;
            }
            Ok(())
        }
        // `fit` is an enum, so serde has already refused anything that is not
        // one of the four spellings; there is nothing left for this to check.
        BackgroundFill::Image {
            media_id,
            opacity,
            blur_px_ratio,
            fit: _,
        } => {
            check_uuid(&join(field, "media_id"), media_id)?;
            check_number(&join(field, "opacity"), *opacity, &UNIT)?;
            check_number(&join(field, "blur_px_ratio"), *blur_px_ratio, &LENGTH)
        }
    }
}

fn check_geometry(field: &str, geometry: &ShapeGeometry) -> Result<(), TemplateError> {
    match geometry {
        ShapeGeometry::Rect { x, y, w, h } | ShapeGeometry::Ellipse { x, y, w, h } => {
            check_box(field, *x, *y, *w, *h)
        }
        ShapeGeometry::RoundedRect {
            x,
            y,
            w,
            h,
            corner_radius,
        } => {
            check_box(field, *x, *y, *w, *h)?;
            check_number(&join(field, "corner_radius"), *corner_radius, &LENGTH)
        }
        ShapeGeometry::Path {
            x,
            y,
            w,
            h,
            d,
            viewbox,
        } => {
            check_box(field, *x, *y, *w, *h)?;
            check_path_data(&join(field, "d"), d)?;
            check_viewbox(&join(field, "viewbox"), viewbox)
        }
    }
}

fn check_typography(field: &str, typography: &Typography) -> Result<(), TemplateError> {
    check_font_family(&join(field, "font_family"), &typography.font_family)?;
    check_int(
        &join(field, "font_fallback"),
        count(typography.font_fallback.len()),
        &FONT_FALLBACKS,
    )?;
    for (index, family) in typography.font_fallback.iter().enumerate() {
        check_font_family(&format!("{field}.font_fallback[{index}]"), family)?;
    }
    check_int(
        &join(field, "weight"),
        u64::from(typography.weight),
        &WEIGHT,
    )?;
    check_number(&join(field, "size"), typography.size, &LENGTH)?;
    check_number(&join(field, "min_size"), typography.min_size, &LENGTH)?;
    // Zero is excluded from both, separately from the range check above,
    // because `LENGTH` has to admit zero for a blur or a stroke and a type size
    // of zero is a different animal: FR-310 divides by the size to work out how
    // many lines fit, so a zero there is an infinity inside the splitter rather
    // than invisible text on a slide.
    //
    // Precondition: both values have been through `check_number` against
    // `LENGTH` above, so a NaN is already refused and does not reach a `<=`
    // that would be false for it.
    for (name, value) in [("size", typography.size), ("min_size", typography.min_size)] {
        if value <= 0.0 {
            return Err(TemplateError::Malformed {
                field: join(field, name),
                reason: "must be greater than zero",
            });
        }
    }
    // A shrink floor above the starting size is not a size, it is a
    // contradiction: FR-310 shrinks *down* to `min_size` before it splits, so
    // this pair would make the splitter's first step grow the text. Refused
    // rather than reconciled, because either repair — raising `size` or
    // lowering `min_size` — changes a design nobody asked to have changed.
    if typography.min_size > typography.size {
        return Err(TemplateError::Malformed {
            field: join(field, "min_size"),
            reason: "must be no larger than size",
        });
    }
    check_number(
        &join(field, "line_height"),
        typography.line_height,
        &LINE_HEIGHT,
    )?;
    check_number(
        &join(field, "letter_spacing"),
        typography.letter_spacing,
        &LETTER_SPACING,
    )?;
    check_colour(&join(field, "color"), &typography.color)?;
    check_int(
        &join(field, "max_lines"),
        u64::from(typography.max_lines),
        &MAX_LINES,
    )
}

fn check_effects(field: &str, effects: &TextEffects) -> Result<(), TemplateError> {
    let shadow = &effects.shadow;
    check_colour(&join(field, "shadow.color"), &shadow.color)?;
    check_number(&join(field, "shadow.opacity"), shadow.opacity, &UNIT)?;
    check_number(&join(field, "shadow.blur"), shadow.blur, &LENGTH)?;
    // Offsets may be negative — a shadow up and to the left is ordinary — so
    // they are bounded like a position rather than like a length.
    check_number(&join(field, "shadow.offset_x"), shadow.offset_x, &POSITION)?;
    check_number(&join(field, "shadow.offset_y"), shadow.offset_y, &POSITION)?;

    let outline = &effects.outline;
    check_colour(&join(field, "outline.color"), &outline.color)?;
    check_number(&join(field, "outline.width"), outline.width, &LENGTH)
}

fn check_rect(field: &str, rect: &Rect) -> Result<(), TemplateError> {
    check_box(field, rect.x, rect.y, rect.w, rect.h)
}

fn check_box(field: &str, x: f64, y: f64, w: f64, h: f64) -> Result<(), TemplateError> {
    check_number(&join(field, "x"), x, &POSITION)?;
    check_number(&join(field, "y"), y, &POSITION)?;
    check_number(&join(field, "w"), w, &EXTENT)?;
    check_number(&join(field, "h"), h, &EXTENT)
}

// ---------------------------------------------------------------------------
// The checks themselves
// ---------------------------------------------------------------------------

/// The span one floating-point setting may take.
struct Bounds {
    min: f64,
    max: f64,
}

/// The span one integer-valued setting, count or length may take.
struct IntBounds {
    min: u64,
    max: u64,
}

fn check_number(field: &str, value: f64, bounds: &Bounds) -> Result<(), TemplateError> {
    // **The frame is positive on purpose, because this is also the NaN check.**
    // Every comparison against NaN is false, so a NaN cannot enter the `Ok`
    // branch and falls out to the error below, instead of reaching a renderer
    // where it would silently become a layout of zero.
    //
    // What carries that property is the *direction* of the frame, not the shape
    // of the comparisons — so read this before rewriting either.
    // `!(bounds.min..=bounds.max).contains(&value)` is the same test, because
    // `RangeBounds::contains` is defined as `start <= item && item <= end`; it
    // refuses NaN exactly as this does, and there is nothing to fear from that
    // refactor. The spelling that *is* dangerous is the natural-looking
    // inverse, `if value < bounds.min || value > bounds.max { Err }`: there
    // both comparisons are false for a NaN, so it misses the error branch and
    // passes.
    //
    // JSON text cannot carry a NaN to begin with — `serde_json` has no `NaN`
    // literal, and refuses an overflowing number such as `1e400` with "number
    // out of range". But `validate_template` also serves documents built in
    // code, where one is perfectly constructible, so the check is not dead.
    if value >= bounds.min && value <= bounds.max {
        return Ok(());
    }
    Err(TemplateError::OutOfRange {
        field: field.to_owned(),
        value,
        min: bounds.min,
        max: bounds.max,
    })
}

fn check_int(field: &str, value: u64, bounds: &IntBounds) -> Result<(), TemplateError> {
    if value >= bounds.min && value <= bounds.max {
        return Ok(());
    }
    Err(TemplateError::IntegerOutOfRange {
        field: field.to_owned(),
        value,
        min: bounds.min,
        max: bounds.max,
    })
}

/// A collection length as the integer the bounds are expressed in.
//
// Saturating rather than truncating. A `usize` wider than `u64` does not exist
// on any target this ships to, so the fallback is unreachable — but if it ever
// were reached, `u64::MAX` fails every bound, which is the safe direction.
fn count(length: usize) -> u64 {
    u64::try_from(length).unwrap_or(u64::MAX)
}

/// Accepts `#RGB` and `#RRGGBB`, and nothing else.
//
// An allowlist, and a deliberately narrow one. Every colour in a template is
// interpolated into CSS by the renderer — an inline `style`, or a `fill`
// attribute on an SVG node — inside the same web origin the Control Panel runs
// in (ADR-0018). Accepting CSS colour *syntax* would mean accepting `url(…)`,
// `var(…)`, and a value carrying a `;` that closes one declaration and opens
// another. Refusing everything but hex makes all of that unrepresentable rather
// than filtered, which is the difference between a rule and a blocklist.
//
// The four- and eight-digit forms are refused too, though browsers take them:
// alpha is a separate field on every colour in Appendix B, and two ways to say
// "half transparent" is one way for them to disagree.
fn check_colour(field: &str, value: &str) -> Result<(), TemplateError> {
    let malformed = || TemplateError::Malformed {
        field: field.to_owned(),
        reason: "must be a hex colour such as #0B0F1A or #000",
    };

    let Some(digits) = value.strip_prefix('#') else {
        return Err(malformed());
    };
    if !matches!(digits.len(), 3 | 6) || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(malformed());
    }
    Ok(())
}

/// Accepts the canonical 8-4-4-4-12 UUID spelling, in either case.
//
// Appendix B types every id in a template as a `uuid`, and this enforces it
// rather than treating it as documentation. The cost is worth naming: a
// hand-written template with `"id": "background"` is refused, so FR-410's
// built-in templates and everything the Builder emits must carry real UUIDs.
// The gain is that an id is a fixed-length string over a fixed alphabet before
// it becomes a DOM id, a database key or part of a filename, so none of those
// consumers has to ask what else it might contain.
//
// The version and variant nibbles are not checked. This is an identity, not a
// claim about how it was generated, and Appendix B's own examples are UUIDv7 —
// a version any stricter check written before it existed would have refused.
fn check_uuid(field: &str, value: &str) -> Result<(), TemplateError> {
    const DASH_POSITIONS: [usize; 4] = [8, 13, 18, 23];
    const UUID_LENGTH: usize = 36;

    let bytes = value.as_bytes();
    let well_formed = bytes.len() == UUID_LENGTH
        && bytes.iter().enumerate().all(|(index, byte)| {
            if DASH_POSITIONS.contains(&index) {
                *byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        });

    if well_formed {
        return Ok(());
    }
    Err(TemplateError::Malformed {
        field: field.to_owned(),
        reason: "must be a UUID such as 018f2c40-7c1e-7a3b-9f10-2b7c5d8e1a01",
    })
}

/// Accepts a font family made of letters, digits, spaces, hyphens and
/// underscores.
//
// The same argument as `check_colour`, one field along: a family name is
// interpolated into a CSS `font-family` declaration. Quotes, semicolons,
// braces, parentheses and backslashes are what an injection needs, and none of
// them appears in a font name, so the allowlist has no room for them. Letters
// are Unicode-aware because a family may legitimately be named in any script.
//
// A leading or trailing space is refused as well: it cannot be seen, and two
// families differing only by one would be indistinguishable in the Builder.
// `is_invisible` is subtracted from the allowlist for the same reason and not
// for a different one: `char::is_alphanumeric` calls U+3164 HANGUL FILLER a
// letter, so `"Inter\u{3164}"` would otherwise pass here and look exactly like
// `Inter` everywhere it is shown.
fn check_font_family(field: &str, value: &str) -> Result<(), TemplateError> {
    let malformed = || TemplateError::Malformed {
        field: field.to_owned(),
        reason: "must be a font family of letters, digits, spaces, hyphens or underscores",
    };

    let length = value.chars().count();
    if length == 0 || length > MAX_FONT_FAMILY_CHARS {
        return Err(malformed());
    }
    if value.starts_with(' ') || value.ends_with(' ') {
        return Err(malformed());
    }
    if !value
        .chars()
        .all(|c| !is_invisible(c) && (c.is_alphanumeric() || matches!(c, ' ' | '-' | '_')))
    {
        return Err(malformed());
    }
    Ok(())
}

/// Accepts SVG path data made only of path commands, numbers and separators.
//
// **This is a character class, not the path grammar, and the difference
// matters.** FR-403 owns the grammar allowlist PRD §6.7 promises: that the
// commands come in a legal order, that each has the right number of arguments,
// that an arc's flags are 0 or 1. None of that is checked here, and a caller
// must not read this as "the path is safe to draw".
//
// What it does buy today is that the string cannot contain `<`, `>`, `"`, `'`,
// `&`, `(` or `\`, so it cannot close an attribute, open a tag, or name a URL
// wherever it is eventually interpolated. That is the property FR-403 builds on
// rather than has to establish, and it is why the field stays a plain `String`:
// a stricter type here would have to encode a grammar this item is explicitly
// not writing, and FR-403 can put a parse step in front of the same field
// without changing the document shape or the generated TypeScript.
fn check_path_data(field: &str, value: &str) -> Result<(), TemplateError> {
    let malformed = || TemplateError::Malformed {
        field: field.to_owned(),
        reason: "must be SVG path data: commands, numbers and separators only",
    };

    let length = value.chars().count();
    if length == 0 || length > MAX_PATH_DATA_CHARS {
        return Err(malformed());
    }
    if !value.chars().all(is_path_char) {
        return Err(malformed());
    }
    Ok(())
}

/// The characters SVG path data is made of: the command letters, the digits,
/// and the punctuation numbers need.
//
// `e` and `E` are here for exponent notation, which is also why they cannot be
// dropped even though they are not commands. Whitespace separates arguments and
// `Zz` closes a subpath. Every other ASCII letter is absent on purpose.
fn is_path_char(c: char) -> bool {
    matches!(
        c,
        'M' | 'm'
            | 'L'
            | 'l'
            | 'H'
            | 'h'
            | 'V'
            | 'v'
            | 'C'
            | 'c'
            | 'S'
            | 's'
            | 'Q'
            | 'q'
            | 'T'
            | 't'
            | 'A'
            | 'a'
            | 'Z'
            | 'z'
            | 'e'
            | 'E'
            | '0'..='9' | '.' | ',' | '+' | '-'
    ) || c.is_ascii_whitespace()
}

/// Accepts a `viewBox`: four numbers, with a positive width and height.
fn check_viewbox(field: &str, value: &str) -> Result<(), TemplateError> {
    let malformed = || TemplateError::Malformed {
        field: field.to_owned(),
        reason: "must be four numbers, as in 0 0 1 1",
    };

    if value.chars().count() > MAX_VIEWBOX_CHARS {
        return Err(malformed());
    }
    let mut numbers = Vec::with_capacity(4);
    for token in value.split([' ', '\t', '\n', '\r', ',']) {
        if token.is_empty() {
            continue;
        }
        let Ok(number) = token.parse::<f64>() else {
            return Err(malformed());
        };
        // `str::parse::<f64>` accepts `inf` and `NaN` by name, so this is **not**
        // a redundant check on a string that already parsed, and deleting it
        // would not be caught anywhere else: the width and height test below is
        // negatively framed, so `"0 0 NaN 1"` would pass it.
        if !number.is_finite() || numbers.len() == 4 {
            return Err(malformed());
        }
        numbers.push(number);
    }
    // A zero or negative width or height makes the viewBox degenerate, and SVG
    // says an element carrying one is not rendered at all — a layer that
    // vanishes with no diagnostic anywhere is exactly what this refuses.
    if numbers.len() != 4 || numbers[2] <= 0.0 || numbers[3] <= 0.0 {
        return Err(malformed());
    }
    Ok(())
}

/// Accepts a timestamp's *shape*: the characters ISO-8601 uses, at a plausible
/// length.
//
// Not a date parser, and the doc comment on `TemplateMetadata::created_at` says
// so to the frontend as well. Writing one by hand would put a calendar in a
// module about templates, and taking a dependency for it would be crates
// against NFR-16 to validate a field nothing renders. What is checked is what
// matters for safety: the string is short and drawn from a tiny alphabet, so it
// cannot smuggle markup or an escape sequence into a diagnostic or a list.
fn check_timestamp(field: &str, value: &str) -> Result<(), TemplateError> {
    let malformed = || TemplateError::Malformed {
        field: field.to_owned(),
        reason: "must be an ISO-8601 timestamp such as 2026-08-07T10:00:00Z",
    };

    let length = value.chars().count();
    if !(MIN_TIMESTAMP_CHARS..=MAX_TIMESTAMP_CHARS).contains(&length) {
        return Err(malformed());
    }
    if !value
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '-' | ':' | 'T' | 'Z' | '.' | '+'))
    {
        return Err(malformed());
    }
    Ok(())
}

/// Accepts a string an operator will read on screen: bounded, free of
/// characters that reorder or hide what follows them, and free of path
/// separators.
///
/// `allow_blank` is the single difference between a template's name and its
/// author.
//
// **Why invisible characters are refused.** They are not a normalisation
// problem, they are a deception: `U+202E` reverses the display order of
// everything after it, so a template named with one appears in the picker as
// something else entirely and the operator picks the wrong design in the middle
// of a service.
//
// The refused set is the whole of `Cc` — `char::is_control` is that test
// exactly — the whole of `Cf`, and the separators `char::is_control` misses;
// `is_invisible` below holds it. Taking the whole of `Cf` rather than a
// hand-listed set of bidi controls is what keeps U+061C ARABIC LETTER MARK in:
// it does exactly what LRM and RLM do, and a second list would be a second
// table to keep in step with the one `scripture::is_invisible` already deletes
// from.
//
// **Why path separators are refused.** `name` is the natural source for an
// `.aerotpl` export filename (FR-409), and no legitimate template name holds
// `/`, `\`, `:` or `..`. Making them unrepresentable here costs nothing and is
// cheaper than depending on FR-409 to remember to strip them.
//
// **Why `<`, `>`, `&`, `"` and `'` are *not* refused.** They are legal in human
// text: "Natal & Tahun Baru" is an ordinary template name, and a validator that
// argues with its operator about punctuation is a defect. The defence against
// markup is escaping at the render boundary, not narrowing here — narrowing
// here would refuse correct input *and* offer false comfort, because this
// function cannot know what its consumer interpolates the string into. That
// obligation sits in the module header's table of what the caller owns, where a
// reader who has just read "colours, ids, font names … allowlists" will meet it.
fn check_display_text(field: &str, value: &str, allow_blank: bool) -> Result<(), TemplateError> {
    let malformed = |reason| TemplateError::Malformed {
        field: field.to_owned(),
        reason,
    };

    // Whitespace-only counts as blank, for the reason
    // `monitor::reported_name` gives: both render as an empty row, and an empty
    // row in a picker is not something an operator can choose between.
    if !allow_blank && value.trim().is_empty() {
        return Err(malformed("must not be blank"));
    }
    if value.chars().count() > MAX_DISPLAY_CHARS {
        return Err(malformed("must be at most 120 characters"));
    }
    if value.chars().any(is_invisible) {
        return Err(malformed(
            "must not contain invisible or text-direction characters",
        ));
    }
    if value.contains(['/', '\\', ':']) || value.contains("..") {
        return Err(malformed("must not contain a path separator"));
    }
    Ok(())
}

/// Whether `c` takes up no space on screen, whatever Unicode calls it.
//
// **Wider than `scripture::is_invisible`, on purpose.** The two share the `Cf`
// table and nothing else: deleting a separator from a book spelling would join
// two words, so that one stops at `Cc`/`Cf`, while refusing one here costs
// nothing. They are not the same predicate and must not be merged into one.
//
// Three groups, and the third is the one that is easy to miss. `Cc` and `Cf`
// are the invisibles `scripture::is_invisible` names, and the `Cf` table is
// shared from `models::text` so the two modules cannot come to disagree about a
// code point. U+2028 and U+2029 are line and paragraph separators, categories
// `Zl` and `Zp`: they break a line without being control characters, so
// `char::is_control` walks straight past them.
//
// The Hangul fillers are the third group. `char::is_alphanumeric` calls each of
// them a letter and every font draws nothing, so `"\u{3164}"` satisfies "must
// not be blank" and is still an empty row in the picker, and `"Inter\u{3164}"`
// is a second font family indistinguishable from `Inter` in the Builder. They
// are refused as *characters* rather than folded into the blank rule because
// `check_font_family` needs the same answer and has no blank rule to fold them
// into: one mechanism covers both call sites, where two would have left the
// font name uncovered.
#[rustfmt::skip]
fn is_invisible(c: char) -> bool {
    c.is_control()
        || text::is_format_char(c)
        || matches!(
            c,
            '\u{2028}'   // LINE SEPARATOR
            | '\u{2029}' // PARAGRAPH SEPARATOR
            | '\u{115f}' // HANGUL CHOSEONG FILLER
            | '\u{1160}' // HANGUL JUNGSEONG FILLER
            | '\u{3164}' // HANGUL FILLER
            | '\u{ffa0}' // HALFWIDTH HANGUL FILLER
        )
}

/// Builds the dotted path a diagnostic names.
//
// Every part of it comes from this module — a field name spelled here, or an
// array index — so a `TemplateError` naming a field never echoes the document
// back at whoever displays it. That is what lets `Malformed` be safe to print
// while `Syntax` is not.
fn join(field: &str, child: &str) -> String {
    format!("{field}.{child}")
}
