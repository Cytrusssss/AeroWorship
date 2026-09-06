use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::text;

pub const SCHEMA_VERSION: u32 = 1;

pub const MAX_DOCUMENT_BYTES: usize = 256 * 1024;

pub const MAX_LAYERS: usize = 32;

const POSITION: Bounds = Bounds {
    min: -0.5,
    max: 1.5,
};

const EXTENT: Bounds = Bounds { min: 0.0, max: 1.5 };

const UNIT: Bounds = Bounds { min: 0.0, max: 1.0 };

const LENGTH: Bounds = Bounds { min: 0.0, max: 1.0 };

const LETTER_SPACING: Bounds = Bounds {
    min: -1.0,
    max: 1.0,
};

const LINE_HEIGHT: Bounds = Bounds {
    min: 0.1,
    max: 10.0,
};

const GRADIENT_ANGLE: Bounds = Bounds {
    min: -360.0,
    max: 360.0,
};

const WEIGHT: IntBounds = IntBounds { min: 1, max: 1000 };

const MAX_LINES: IntBounds = IntBounds { min: 1, max: 64 };

const REFERENCE_DIMENSION: IntBounds = IntBounds {
    min: 1,
    max: 16_384,
};

const GRADIENT_STOPS: IntBounds = IntBounds { min: 2, max: 16 };

const FONT_FALLBACKS: IntBounds = IntBounds { min: 0, max: 8 };

const MAX_FONT_FAMILY_CHARS: usize = 64;

const MAX_DISPLAY_CHARS: usize = 120;

const MAX_PATH_DATA_CHARS: usize = 4096;

const MAX_VIEWBOX_CHARS: usize = 64;

const MIN_TIMESTAMP_CHARS: usize = 20;
const MAX_TIMESTAMP_CHARS: usize = 32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct TemplateDocument {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub canvas: Canvas,
    pub layers: Vec<Layer>,
    pub safe_area: Rect,
    pub metadata: TemplateMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct Canvas {
    pub aspect_ratio: AspectRatio,
    pub reference_width: u32,
    pub reference_height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum AspectRatio {
    #[serde(rename = "16:9")]
    SixteenNine,
    #[serde(rename = "16:10")]
    SixteenTen,
    #[serde(rename = "4:3")]
    FourThree,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum Layer {
    Background {
        id: String,
        visible: bool,
        fill: BackgroundFill,
    },
    Shape {
        id: String,
        visible: bool,
        geometry: ShapeGeometry,
        fill: ShapeFill,
        stroke: Stroke,
        blur_px_ratio: f64,
    },
    Text {
        id: String,
        visible: bool,
        role: TextRole,
        #[serde(rename = "box")]
        text_box: Rect,
        typography: Typography,
        effects: TextEffects,
    },
}

impl Layer {
    pub fn id(&self) -> &str {
        match self {
            Self::Background { id, .. } | Self::Shape { id, .. } | Self::Text { id, .. } => id,
        }
    }

    pub fn is_visible(&self) -> bool {
        match self {
            Self::Background { visible, .. }
            | Self::Shape { visible, .. }
            | Self::Text { visible, .. } => *visible,
        }
    }

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum BackgroundFill {
    Solid {
        color: String,
    },
    Gradient {
        angle_deg: f64,
        stops: Vec<GradientStop>,
    },
    Image {
        media_id: String,
        fit: ImageFit,
        opacity: f64,
        blur_px_ratio: f64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct GradientStop {
    pub offset: f64,
    pub color: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum ImageFit {
    Cover,
    Contain,
    Stretch,
    Tile,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum ShapeGeometry {
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    RoundedRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        corner_radius: f64,
    },
    Ellipse {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    Path {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        d: String,
        viewbox: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct ShapeFill {
    pub color: String,
    pub opacity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct Stroke {
    pub color: String,
    pub width: f64,
    pub opacity: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum TextRole {
    Primary,
    Secondary,
    Reference,
    Attribution,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct Typography {
    pub font_family: String,
    pub font_fallback: Vec<String>,
    pub weight: u16,
    pub size: f64,
    pub min_size: f64,
    pub line_height: f64,
    pub letter_spacing: f64,
    pub transform: TextTransform,
    pub color: String,
    pub align_h: HorizontalAlign,
    pub align_v: VerticalAlign,
    pub max_lines: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum TextTransform {
    None,
    Uppercase,
    Capitalize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct TextEffects {
    pub shadow: ShadowEffect,
    pub outline: OutlineEffect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct ShadowEffect {
    pub enabled: bool,
    pub color: String,
    pub opacity: f64,
    pub blur: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct OutlineEffect {
    pub enabled: bool,
    pub color: String,
    pub width: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(test, derive(ts_rs::TS))]
#[cfg_attr(test, ts(export))]
pub struct TemplateMetadata {
    pub created_at: String,
    pub updated_at: String,
    pub author: String,
}

#[derive(Debug)]
pub enum TemplateError {
    TooLarge {
        bytes: usize,
        limit: usize,
    },

    Syntax(serde_json::Error),

    UnsupportedSchemaVersion {
        found: u64,
        supported: u32,
    },

    OutOfRange {
        field: String,
        value: f64,
        min: f64,
        max: f64,
    },

    IntegerOutOfRange {
        field: String,
        value: u64,
        min: u64,
        max: u64,
    },

    Malformed {
        field: String,
        reason: &'static str,
    },

    DuplicateLayerId {
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

pub fn parse_template(json: &str) -> Result<TemplateDocument, TemplateError> {
    if json.len() > MAX_DOCUMENT_BYTES {
        return Err(TemplateError::TooLarge {
            bytes: json.len(),
            limit: MAX_DOCUMENT_BYTES,
        });
    }

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

fn probe_schema_version(json: &str) -> Option<u64> {
    #[derive(Deserialize)]
    struct VersionProbe {
        schema_version: u64,
    }

    serde_json::from_str::<VersionProbe>(json)
        .ok()
        .map(|probe| probe.schema_version)
}

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
    for (name, value) in [("size", typography.size), ("min_size", typography.min_size)] {
        if value <= 0.0 {
            return Err(TemplateError::Malformed {
                field: join(field, name),
                reason: "must be greater than zero",
            });
        }
    }
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

struct Bounds {
    min: f64,
    max: f64,
}

struct IntBounds {
    min: u64,
    max: u64,
}

fn check_number(field: &str, value: f64, bounds: &Bounds) -> Result<(), TemplateError> {
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

fn count(length: usize) -> u64 {
    u64::try_from(length).unwrap_or(u64::MAX)
}

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
        if !number.is_finite() || numbers.len() == 4 {
            return Err(malformed());
        }
        numbers.push(number);
    }
    if numbers.len() != 4 || numbers[2] <= 0.0 || numbers[3] <= 0.0 {
        return Err(malformed());
    }
    Ok(())
}

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

fn check_display_text(field: &str, value: &str, allow_blank: bool) -> Result<(), TemplateError> {
    let malformed = |reason| TemplateError::Malformed {
        field: field.to_owned(),
        reason,
    };

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

fn join(field: &str, child: &str) -> String {
    format!("{field}.{child}")
}
