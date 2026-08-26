//! Loading a template document (FR-401, PRD Appendix B).
//!
//! FR-401's acceptance criterion is two clauses, and each has its own section
//! below: *"a saved template validates against Appendix B"* is
//! [`APPENDIX_B_2`], copied character for character out of `docs/PRD.md`
//! (Appendix B §B.2) rather than retyped, and round-tripped so that a parser
//! which quietly dropped a field could not pass; *"a template containing a
//! script node is rejected on load"* is the `script`/`iframe`/`webview` group,
//! plus the `on*`-attribute group beside it — because a payload that cannot
//! arrive as a layer `type` will try to arrive as an extra key on one.
//!
//! Everything else here is written to fail a specific wrong implementation,
//! not to agree with the current one. Named, so a later reader can check they
//! still fail:
//!
//! - dropping `deny_unknown_fields` from the document, or from any of the
//!   three internally-tagged enums (`Layer`, `BackgroundFill`,
//!   `ShapeGeometry`) — serde ignores unknown fields by default, so this is a
//!   silent regression with no compile error behind it;
//! - deleting the lenient `schema_version` probe, which turns "your file is
//!   newer than this build" into "your file is corrupt";
//! - clamping an out-of-range coordinate instead of refusing it;
//! - spelling the range check as "refuse when `value < min || value > max`",
//!   which is true of no NaN and therefore lets one through;
//! - dropping the layer cap or the document byte cap;
//! - loosening the colour rule to "starts with `#`";
//! - dropping the layer-id uniqueness check, or comparing ids case-sensitively.
//!
//! **Every fixture but `APPENDIX_B_2` is synthetic.** The ids are sequential
//! placeholders, the names are obviously invented, and no lyric, verse or
//! other real content appears anywhere: a template carries no text at all
//! (PRD §6.7), only the slots text later lands in.

use aeroworship_core::models::template::{MAX_DOCUMENT_BYTES, MAX_LAYERS, SCHEMA_VERSION};
use aeroworship_core::models::{
    parse_template, validate_template, AspectRatio, BackgroundFill, ImageFit, Layer, ShapeGeometry,
    TemplateDocument, TemplateError, TextRole,
};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Appendix B §B.2, verbatim
//
// Copied out of `docs/PRD.md` lines 1379-1452 unchanged, including its
// whitespace. If the appendix moves, this constant is what has to be recopied;
// rewriting it from memory is what the criterion is guarding against.
// ---------------------------------------------------------------------------

/// The worked example "Photo with Scrim", exactly as Appendix B §B.2 prints it.
const APPENDIX_B_2: &str = r##"
{
  "schema_version": 1,
  "id": "018f2c40-7c1e-7a3b-9f10-2b7c5d8e1a01",
  "name": "Photo with Scrim",
  "canvas": { "aspect_ratio": "16:9", "reference_width": 1920, "reference_height": 1080 },
  "layers": [
    {
      "id": "018f2c40-7c1e-7a3b-9f10-2b7c5d8e1a02",
      "type": "background",
      "visible": true,
      "fill": {
        "kind": "image",
        "media_id": "018f2c3f-1111-7000-8000-000000000001",
        "fit": "cover",
        "opacity": 1.0,
        "blur_px_ratio": 0.004
      }
    },
    {
      "id": "018f2c40-7c1e-7a3b-9f10-2b7c5d8e1a03",
      "type": "shape",
      "visible": true,
      "geometry": { "kind": "rect", "x": 0.0, "y": 0.48, "w": 1.0, "h": 0.52 },
      "fill": { "color": "#000000", "opacity": 0.60 },
      "stroke": { "color": "#000000", "width": 0.0, "opacity": 0.0 },
      "blur_px_ratio": 0.0
    },
    {
      "id": "018f2c40-7c1e-7a3b-9f10-2b7c5d8e1a04",
      "type": "text",
      "visible": true,
      "role": "primary",
      "box": { "x": 0.08, "y": 0.55, "w": 0.84, "h": 0.32 },
      "typography": {
        "font_family": "Inter",
        "font_fallback": ["Segoe UI", "sans-serif"],
        "weight": 700, "size": 0.072, "min_size": 0.048,
        "line_height": 1.24, "letter_spacing": 0.0, "transform": "none",
        "color": "#FFFFFF", "align_h": "center", "align_v": "middle",
        "max_lines": 4
      },
      "effects": {
        "shadow": { "enabled": true, "color": "#000000", "opacity": 0.75,
                    "blur": 0.006, "offset_x": 0.0, "offset_y": 0.002 },
        "outline": { "enabled": false, "color": "#000000", "width": 0.002 }
      }
    },
    {
      "id": "018f2c40-7c1e-7a3b-9f10-2b7c5d8e1a05",
      "type": "text",
      "visible": true,
      "role": "attribution",
      "box": { "x": 0.08, "y": 0.90, "w": 0.84, "h": 0.06 },
      "typography": {
        "font_family": "Inter", "font_fallback": ["Segoe UI", "sans-serif"],
        "weight": 400, "size": 0.020, "min_size": 0.016,
        "line_height": 1.2, "letter_spacing": 0.01, "transform": "none",
        "color": "#D8DEE9", "align_h": "center", "align_v": "middle",
        "max_lines": 2
      },
      "effects": {
        "shadow": { "enabled": false, "color": "#000000", "opacity": 0.0,
                    "blur": 0.0, "offset_x": 0.0, "offset_y": 0.0 },
        "outline": { "enabled": false, "color": "#000000", "width": 0.0 }
      }
    }
  ],
  "safe_area": { "x": 0.05, "y": 0.05, "w": 0.90, "h": 0.90 },
  "metadata": {
    "created_at": "2026-08-07T10:00:00Z",
    "updated_at": "2026-08-07T10:00:00Z",
    "author": "Rio"
  }
}
"##;

// ---------------------------------------------------------------------------
// Synthetic fixtures and the small amount of machinery around them
// ---------------------------------------------------------------------------

const DOC_ID: &str = "0191aaaa-0000-7000-8000-000000000001";
const BACKGROUND_ID: &str = "0191aaaa-0000-7000-8000-000000000002";
const SHAPE_ID: &str = "0191aaaa-0000-7000-8000-000000000003";
const TEXT_ID: &str = "0191aaaa-0000-7000-8000-000000000004";
const MEDIA_ID: &str = "0191aaaa-0000-7000-8000-0000000000ff";

/// A document that satisfies every rule, built here rather than taken from the
/// appendix so that a test which breaks one field is unambiguous about which.
///
/// Layer 0 is a solid background, layer 1 a rectangular scrim, layer 2 a text
/// slot; the pointers below index them in that order.
fn sound() -> Value {
    json!({
        "schema_version": 1,
        "id": DOC_ID,
        "name": "Fixture Template",
        "canvas": { "aspect_ratio": "16:9", "reference_width": 1920, "reference_height": 1080 },
        "layers": [
            {
                "id": BACKGROUND_ID,
                "type": "background",
                "visible": true,
                "fill": { "kind": "solid", "color": "#0B0F1A" }
            },
            {
                "id": SHAPE_ID,
                "type": "shape",
                "visible": true,
                "geometry": { "kind": "rect", "x": 0.0, "y": 0.48, "w": 1.0, "h": 0.52 },
                "fill": { "color": "#000000", "opacity": 0.6 },
                "stroke": { "color": "#000000", "width": 0.0, "opacity": 0.0 },
                "blur_px_ratio": 0.0
            },
            {
                "id": TEXT_ID,
                "type": "text",
                "visible": true,
                "role": "primary",
                "box": { "x": 0.08, "y": 0.55, "w": 0.84, "h": 0.32 },
                "typography": {
                    "font_family": "Inter",
                    "font_fallback": ["Segoe UI", "sans-serif"],
                    "weight": 700,
                    "size": 0.072,
                    "min_size": 0.048,
                    "line_height": 1.24,
                    "letter_spacing": 0.0,
                    "transform": "none",
                    "color": "#FFFFFF",
                    "align_h": "center",
                    "align_v": "middle",
                    "max_lines": 4
                },
                "effects": {
                    "shadow": { "enabled": true, "color": "#000000", "opacity": 0.75,
                                "blur": 0.006, "offset_x": 0.0, "offset_y": 0.002 },
                    "outline": { "enabled": false, "color": "#000000", "width": 0.002 }
                }
            }
        ],
        "safe_area": { "x": 0.05, "y": 0.05, "w": 0.9, "h": 0.9 },
        "metadata": {
            "created_at": "2026-08-07T10:00:00Z",
            "updated_at": "2026-08-07T10:00:00Z",
            "author": "Fixture Author"
        }
    })
}

/// A gradient background fill, for the fixtures that need one.
fn gradient_fill() -> Value {
    json!({
        "kind": "gradient",
        "angle_deg": 180.0,
        "stops": [
            { "offset": 0.0, "color": "#000" },
            { "offset": 1.0, "color": "#FFF" }
        ]
    })
}

/// An image background fill.
fn image_fill() -> Value {
    json!({
        "kind": "image",
        "media_id": MEDIA_ID,
        "fit": "cover",
        "opacity": 1.0,
        "blur_px_ratio": 0.004
    })
}

/// A path geometry, with `d` and `viewbox` an author might plausibly write.
fn path_geometry() -> Value {
    json!({
        "kind": "path",
        "x": 0.1, "y": 0.1, "w": 0.8, "h": 0.8,
        "d": "M0,0 L1,0 L1,1 Z",
        "viewbox": "0 0 1 1"
    })
}

/// `sound()` with one JSON-pointer location replaced.
fn with(pointer: &str, value: Value) -> Value {
    let mut document = sound();
    let slot = document
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("{pointer} is not a location in the fixture"));
    *slot = value;
    document
}

/// `sound()` with an extra key added to the object at `pointer` (`""` is the
/// document itself).
fn with_extra(pointer: &str, key: &str, value: Value) -> Value {
    let mut document = sound();
    let slot = document
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("{pointer} is not a location in the fixture"))
        .as_object_mut()
        .unwrap_or_else(|| panic!("{pointer} is not an object in the fixture"));
    slot.insert(key.to_owned(), value);
    document
}

/// The document, as the text `parse_template` is handed.
fn text(document: &Value) -> String {
    serde_json::to_string(document).expect("fixture serialises")
}

/// Loads a document that is expected to load.
#[track_caller]
fn accepted(document: &Value) -> TemplateDocument {
    match parse_template(&text(document)) {
        Ok(loaded) => loaded,
        Err(err) => panic!("expected this document to load, but it was refused: {err}"),
    }
}

/// Loads a document that is expected to be refused, and hands back why.
#[track_caller]
fn refused(document: &Value) -> TemplateError {
    match parse_template(&text(document)) {
        Ok(_) => panic!("expected this document to be refused, but it loaded"),
        Err(err) => err,
    }
}

/// Asserts the refusal came from serde — a wrong shape, an unknown field or an
/// unknown variant — and quotes the offending name.
#[track_caller]
fn assert_syntax(err: &TemplateError, needle: &str) {
    let TemplateError::Syntax(inner) = err else {
        panic!("expected a Syntax error mentioning {needle:?}, got {err:?}");
    };
    let message = inner.to_string();
    assert!(
        message.contains(needle),
        "expected the parse error to mention {needle:?}, got {message:?}"
    );
}

/// Asserts the refusal named a field as out of range.
#[track_caller]
fn assert_out_of_range(err: &TemplateError, field: &str) {
    let TemplateError::OutOfRange { field: named, .. } = err else {
        panic!("expected {field} to be out of range, got {err:?}");
    };
    assert_eq!(named, field, "the wrong field was named");
}

/// Asserts the refusal named a field as malformed.
#[track_caller]
fn assert_malformed(err: &TemplateError, field: &str) {
    let TemplateError::Malformed { field: named, .. } = err else {
        panic!("expected {field} to be malformed, got {err:?}");
    };
    assert_eq!(named, field, "the wrong field was named");
}

// ---------------------------------------------------------------------------
// "A saved template validates against Appendix B"
// ---------------------------------------------------------------------------

#[test]
fn the_appendix_b_2_worked_example_loads() {
    let document = parse_template(APPENDIX_B_2).expect("Appendix B §B.2 must load");

    assert_eq!(document.schema_version, SCHEMA_VERSION);
    assert_eq!(document.id, "018f2c40-7c1e-7a3b-9f10-2b7c5d8e1a01");
    assert_eq!(document.name, "Photo with Scrim");
    assert_eq!(document.canvas.aspect_ratio, AspectRatio::SixteenNine);
    assert_eq!(document.canvas.reference_width, 1920);
    assert_eq!(document.canvas.reference_height, 1080);
    assert_eq!(document.layers.len(), 4);
    assert_eq!(document.metadata.author, "Rio");
}

#[test]
fn the_appendix_b_2_layers_keep_their_order_and_their_contents() {
    let document = parse_template(APPENDIX_B_2).expect("Appendix B §B.2 must load");

    // Index 0 is the back layer, and it is the photograph.
    let Layer::Background { id, visible, fill } = &document.layers[0] else {
        panic!("layer 0 of §B.2 is a background layer");
    };
    assert_eq!(id, "018f2c40-7c1e-7a3b-9f10-2b7c5d8e1a02");
    assert!(visible);
    let BackgroundFill::Image {
        media_id,
        fit,
        opacity,
        blur_px_ratio,
    } = fill
    else {
        panic!("§B.2's background is an image fill");
    };
    assert_eq!(media_id, "018f2c3f-1111-7000-8000-000000000001");
    assert_eq!(*fit, ImageFit::Cover);
    assert_eq!(*opacity, 1.0);
    assert_eq!(*blur_px_ratio, 0.004);
    assert_eq!(
        document.layers[0].media_id(),
        Some("018f2c3f-1111-7000-8000-000000000001"),
        "the media reference has to be reachable, because the caller checks it \
         against the permitted roots"
    );

    // Index 1 is the scrim: a plain rect, so it carries no corner radius.
    let Layer::Shape {
        geometry,
        fill,
        blur_px_ratio,
        ..
    } = &document.layers[1]
    else {
        panic!("layer 1 of §B.2 is a shape layer");
    };
    assert_eq!(
        *geometry,
        ShapeGeometry::Rect {
            x: 0.0,
            y: 0.48,
            w: 1.0,
            h: 0.52
        }
    );
    assert_eq!(fill.opacity, 0.60);
    assert_eq!(*blur_px_ratio, 0.0);

    // Indices 2 and 3 are the two text slots, bound to different roles.
    let Layer::Text {
        role, typography, ..
    } = &document.layers[2]
    else {
        panic!("layer 2 of §B.2 is a text layer");
    };
    assert_eq!(*role, TextRole::Primary);
    assert_eq!(typography.font_family, "Inter");
    assert_eq!(typography.font_fallback, ["Segoe UI", "sans-serif"]);
    assert_eq!(typography.weight, 700);
    assert_eq!(typography.size, 0.072);
    assert_eq!(typography.min_size, 0.048);
    assert_eq!(typography.max_lines, 4);

    let Layer::Text { role, effects, .. } = &document.layers[3] else {
        panic!("layer 3 of §B.2 is a text layer");
    };
    assert_eq!(*role, TextRole::Attribution);
    assert!(!effects.shadow.enabled);
    assert!(!effects.outline.enabled);
}

#[test]
fn the_appendix_b_2_worked_example_round_trips() {
    let once = parse_template(APPENDIX_B_2).expect("Appendix B §B.2 must load");
    let written = serde_json::to_string(&once).expect("a loaded template must serialise");
    let twice = parse_template(&written).expect("what this build writes, this build must read");

    assert_eq!(
        once, twice,
        "serialising and reloading changed the document; a field was dropped or renamed"
    );
    assert!(
        validate_template(&twice).is_ok(),
        "the reloaded document must still satisfy Appendix B"
    );
}

#[test]
fn a_saved_template_keeps_appendix_b_s_own_key_spellings() {
    // The round-trip above would still pass if every key were renamed, as long
    // as it were renamed consistently. This is the check that the text written
    // to disk is the text Appendix B describes — including `box`, which is a
    // Rust keyword and is therefore the one field most likely to drift.
    let document = parse_template(APPENDIX_B_2).expect("Appendix B §B.2 must load");
    let written: Value = serde_json::from_str(&serde_json::to_string(&document).unwrap()).unwrap();

    for key in [
        "schema_version",
        "id",
        "name",
        "canvas",
        "layers",
        "safe_area",
        "metadata",
    ] {
        assert!(written.get(key).is_some(), "the saved document lost {key}");
    }
    assert_eq!(written["canvas"]["aspect_ratio"], json!("16:9"));
    assert_eq!(written["layers"][0]["type"], json!("background"));
    assert_eq!(written["layers"][0]["fill"]["kind"], json!("image"));
    assert_eq!(written["layers"][1]["geometry"]["kind"], json!("rect"));
    assert!(
        written["layers"][1]["geometry"]
            .get("corner_radius")
            .is_none(),
        "a plain rect must not gain a corner radius on the way out"
    );
    assert_eq!(written["layers"][2]["type"], json!("text"));
    assert!(
        written["layers"][2].get("box").is_some(),
        "the text slot is spelled `box` in Appendix B"
    );
    assert!(
        written["layers"][2].get("text_box").is_none(),
        "`text_box` is the Rust field name and must not reach the file"
    );
}

#[test]
fn the_synthetic_fixture_is_itself_a_valid_document() {
    // Everything below mutates `sound()`. If `sound()` were already invalid,
    // every one of those tests would pass for the wrong reason.
    let document = accepted(&sound());
    assert_eq!(document.layers.len(), 3);
    assert!(validate_template(&document).is_ok());
}

#[test]
fn every_background_fill_and_every_geometry_in_appendix_b_loads() {
    // §B.2 exercises one fill and one geometry. The other spellings Appendix
    // B §B.1 lists have to load too, or half the schema is untested.
    for fill in [gradient_fill(), image_fill()] {
        accepted(&with("/layers/0/fill", fill));
    }
    for fit in ["cover", "contain", "stretch", "tile"] {
        let mut fill = image_fill();
        fill["fit"] = json!(fit);
        accepted(&with("/layers/0/fill", fill));
    }
    for geometry in [
        json!({ "kind": "rect", "x": 0.0, "y": 0.0, "w": 1.0, "h": 1.0 }),
        json!({ "kind": "rounded_rect", "x": 0.0, "y": 0.0, "w": 1.0, "h": 1.0,
                "corner_radius": 0.02 }),
        json!({ "kind": "ellipse", "x": 0.0, "y": 0.0, "w": 1.0, "h": 1.0 }),
        path_geometry(),
    ] {
        accepted(&with("/layers/1/geometry", geometry));
    }
    for role in ["primary", "secondary", "reference", "attribution"] {
        accepted(&with("/layers/2/role", json!(role)));
    }
    for transform in ["none", "uppercase", "capitalize"] {
        accepted(&with("/layers/2/typography/transform", json!(transform)));
    }
    for ratio in ["16:9", "16:10", "4:3"] {
        accepted(&with("/canvas/aspect_ratio", json!(ratio)));
    }
}

#[test]
fn a_template_with_no_layers_at_all_is_accepted() {
    // A deliberate decision, and one worth pinning: Appendix B caps layers at
    // 32 and sets no floor, and an empty canvas is a legal thing to save
    // halfway through authoring one.
    let document = accepted(&with("/layers", json!([])));
    assert!(document.layers.is_empty());
}

// ---------------------------------------------------------------------------
// "A template containing a script node is rejected on load"
// ---------------------------------------------------------------------------

#[test]
fn a_layer_whose_type_is_script_is_rejected() {
    let err = refused(&with(
        "/layers/2",
        json!({
            "id": TEXT_ID,
            "type": "script",
            "visible": true,
            "src": "https://example.invalid/x.js"
        }),
    ));
    assert_syntax(&err, "unknown variant");
    assert_syntax(&err, "script");
}

#[test]
fn no_layer_type_outside_the_three_appendix_b_names_is_accepted() {
    // The refusal is general — an unrecognised tag is not a variant — rather
    // than a rule about the word "script", and these are the neighbours that
    // prove it.
    for spelling in [
        "script",
        "iframe",
        "webview",
        "foreignObject",
        "html",
        "embed",
        "object",
        "svg",
        "Script",
        "SCRIPT",
        "",
    ] {
        let mut layer = json!({ "id": TEXT_ID, "visible": true });
        layer["type"] = json!(spelling);
        let err = refused(&with("/layers/2", layer));
        assert_syntax(&err, "unknown variant");
    }
}

#[test]
fn a_background_fill_kind_outside_appendix_b_is_rejected() {
    for spelling in ["script", "url", "iframe", "video"] {
        let err = refused(&with(
            "/layers/0/fill",
            json!({ "kind": spelling, "color": "#000" }),
        ));
        assert_syntax(&err, "unknown variant");
    }
}

#[test]
fn a_shape_geometry_kind_outside_appendix_b_is_rejected() {
    for spelling in ["script", "foreignObject", "image", "use"] {
        let err = refused(&with(
            "/layers/1/geometry",
            json!({ "kind": spelling, "x": 0.0, "y": 0.0, "w": 1.0, "h": 1.0 }),
        ));
        assert_syntax(&err, "unknown variant");
    }
}

#[test]
fn a_layer_with_no_type_at_all_is_rejected() {
    let mut layer = sound()["layers"][2].clone();
    layer.as_object_mut().unwrap().remove("type");
    let err = refused(&with("/layers/2", layer));
    assert_syntax(&err, "type");
}

// ---------------------------------------------------------------------------
// Unknown fields: the other shape "no executable code" has to take
//
// `on*` handlers and `script` payloads that cannot arrive as a layer *type*
// will arrive as an extra *key*, and serde ignores unknown keys unless it is
// told not to. Every level of the document is checked, and the three
// internally-tagged enums are checked individually because `ts-rs` reports
// "failed to parse serde attribute: deny_unknown_fields" for exactly those
// three — a warning whose wording does not say whether serde's own
// enforcement survived it.
// ---------------------------------------------------------------------------

#[test]
fn an_unknown_field_on_the_document_is_rejected() {
    for (key, value) in [
        ("onLoad", json!("alert(1)")),
        ("script", json!("alert(1)")),
        ("href", json!("javascript:alert(1)")),
        ("__proto__", json!({ "polluted": true })),
    ] {
        let err = refused(&with_extra("", key, value));
        assert_syntax(&err, "unknown field");
        assert_syntax(&err, key);
    }
}

#[test]
fn an_unknown_field_on_a_tagged_layer_variant_is_rejected() {
    // `Layer` is one of the three enums `ts-rs` warns about.
    for (pointer, key) in [
        ("/layers/0", "onLoad"),
        ("/layers/0", "script"),
        ("/layers/1", "onClick"),
        ("/layers/1", "href"),
        ("/layers/2", "onLoad"),
        ("/layers/2", "innerHTML"),
    ] {
        let err = refused(&with_extra(pointer, key, json!("alert(1)")));
        assert_syntax(&err, "unknown field");
        assert_syntax(&err, key);
    }
}

#[test]
fn an_unknown_field_on_a_tagged_background_fill_variant_is_rejected() {
    // `BackgroundFill` is the second of the three.
    let err = refused(&with_extra("/layers/0/fill", "script", json!("alert(1)")));
    assert_syntax(&err, "unknown field");
    assert_syntax(&err, "script");

    let mut document = with("/layers/0/fill", gradient_fill());
    document["layers"][0]["fill"]["script"] = json!("alert(1)");
    let err = refused(&document);
    assert_syntax(&err, "unknown field");
    assert_syntax(&err, "script");

    let mut document = with("/layers/0/fill", image_fill());
    document["layers"][0]["fill"]["href"] = json!("javascript:alert(1)");
    let err = refused(&document);
    assert_syntax(&err, "unknown field");
    assert_syntax(&err, "href");
}

#[test]
fn an_unknown_field_on_a_tagged_shape_geometry_variant_is_rejected() {
    // `ShapeGeometry` is the third.
    let err = refused(&with_extra("/layers/1/geometry", "xss", json!(1)));
    assert_syntax(&err, "unknown field");
    assert_syntax(&err, "xss");

    let mut document = with(
        "/layers/1/geometry",
        json!({ "kind": "rounded_rect", "x": 0.0, "y": 0.0, "w": 1.0, "h": 1.0,
                "corner_radius": 0.02 }),
    );
    document["layers"][1]["geometry"]["xss"] = json!(1);
    let err = refused(&document);
    assert_syntax(&err, "unknown field");
    assert_syntax(&err, "xss");

    let mut document = with("/layers/1/geometry", path_geometry());
    document["layers"][1]["geometry"]["onLoad"] = json!("alert(1)");
    let err = refused(&document);
    assert_syntax(&err, "unknown field");
    assert_syntax(&err, "onLoad");
}

#[test]
fn corner_radius_on_a_plain_rect_is_rejected() {
    // Appendix B says "rounded_rect only", and this is the case that says so
    // in a way a reader of the JSON would not expect: the key is legal
    // *somewhere* in the schema, which is exactly the mistake
    // `deny_unknown_fields` on the tagged enum has to catch.
    let err = refused(&with_extra(
        "/layers/1/geometry",
        "corner_radius",
        json!(0.02),
    ));
    assert_syntax(&err, "unknown field");
    assert_syntax(&err, "corner_radius");
}

#[test]
fn an_unknown_field_on_a_nested_struct_is_rejected() {
    for (pointer, key) in [
        ("/canvas", "onLoad"),
        ("/safe_area", "onLoad"),
        ("/metadata", "script"),
        ("/layers/2/box", "onLoad"),
        ("/layers/2/typography", "onLoad"),
        ("/layers/2/typography", "font_url"),
        ("/layers/2/effects", "script"),
        ("/layers/2/effects/shadow", "onLoad"),
        ("/layers/2/effects/outline", "onLoad"),
        ("/layers/1/fill", "onLoad"),
        ("/layers/1/stroke", "onLoad"),
    ] {
        let err = refused(&with_extra(pointer, key, json!("alert(1)")));
        assert_syntax(&err, "unknown field");
        assert_syntax(&err, key);
    }
}

#[test]
fn an_unknown_field_on_a_gradient_stop_is_rejected() {
    let mut document = with("/layers/0/fill", gradient_fill());
    document["layers"][0]["fill"]["stops"][1]["onLoad"] = json!("alert(1)");
    let err = refused(&document);
    assert_syntax(&err, "unknown field");
    assert_syntax(&err, "onLoad");
}

// ---------------------------------------------------------------------------
// `schema_version`
//
// The mechanism, not just the wording: the version is read by a lenient probe
// *before* the strict deserialise, so a document this build cannot possibly
// fit is still reported as a version mismatch. Each test below therefore uses
// a document the strict pass would reject for some other reason, because a
// document that happens to fit both versions cannot tell the two orderings
// apart.
// ---------------------------------------------------------------------------

#[test]
fn a_newer_schema_version_is_reported_as_a_version_and_not_as_a_parse_error() {
    let mut document = with_extra("", "tint_profile", json!({ "gamut": "p3" }));
    document["schema_version"] = json!(2);

    let err = refused(&document);
    let TemplateError::UnsupportedSchemaVersion { found, supported } = err else {
        panic!(
            "a version 2 document must be refused as a version, not as {err:?} — \
             the lenient probe has to run before the strict deserialise"
        );
    };
    assert_eq!(found, 2);
    assert_eq!(supported, SCHEMA_VERSION);

    let message = TemplateError::UnsupportedSchemaVersion { found, supported }.to_string();
    assert!(
        message.contains("newer version"),
        "the message must say the file is newer, got {message:?}"
    );
    assert!(
        !message.contains("tint_profile"),
        "the operator must not be sent looking for a corrupt field, got {message:?}"
    );
}

#[test]
fn a_newer_schema_version_wins_over_a_document_that_could_never_deserialise() {
    // Nothing here fits this build's types at all. The version is still what
    // is reported, which is only possible if the probe precedes the parse.
    let json = r#"{ "schema_version": 7, "everything": "else", "layers": "not an array" }"#;
    match parse_template(json) {
        Err(TemplateError::UnsupportedSchemaVersion { found, .. }) => assert_eq!(found, 7),
        other => panic!("expected a version 7 diagnostic, got {other:?}"),
    }
}

#[test]
fn schema_version_zero_gets_a_different_message_and_is_still_not_a_parse_error() {
    let mut document = with_extra("", "legacy_palette", json!(["#000"]));
    document["schema_version"] = json!(0);

    let err = refused(&document);
    let TemplateError::UnsupportedSchemaVersion { found, supported } = err else {
        panic!("a version 0 document must be refused as a version, not as {err:?}");
    };
    assert_eq!(found, 0);
    assert_eq!(supported, SCHEMA_VERSION);

    let older = TemplateError::UnsupportedSchemaVersion { found, supported }.to_string();
    let newer = TemplateError::UnsupportedSchemaVersion {
        found: 2,
        supported,
    }
    .to_string();
    assert_ne!(
        older, newer,
        "\"older than this build\" and \"newer than this build\" are different situations \
         and must not print the same sentence"
    );
    assert!(
        !older.contains("newer version"),
        "a version 0 file is not newer, got {older:?}"
    );
    assert!(older.contains('0'), "the message must name the version");
}

#[test]
fn a_document_with_no_schema_version_falls_through_to_the_parse_error() {
    // The probe fails, and it is meant to: with no version to report, serde's
    // "missing field" is the better diagnostic.
    let mut document = sound();
    document.as_object_mut().unwrap().remove("schema_version");
    let err = refused(&document);
    assert_syntax(&err, "schema_version");
}

#[test]
fn a_schema_version_that_is_not_a_whole_number_falls_through_to_the_parse_error() {
    // The probe reads a `u64` or nothing, so none of these is a version it can
    // report. What is asserted is the fall-through itself: the strict pass owns
    // the diagnostic. (`serde_json` names the *type* it wanted rather than the
    // field for a wrong-typed value, so only the variant can be pinned here.)
    for spelling in [json!("1"), json!(1.5), json!(null), json!(-1), json!(true)] {
        let err = refused(&with("/schema_version", spelling.clone()));
        assert!(
            matches!(err, TemplateError::Syntax(_)),
            "{spelling} is not a version this build can report; it must fall              through to the parse error, got {err:?}"
        );
    }
}

#[test]
fn validate_template_refuses_a_wrong_version_on_a_document_it_did_not_parse() {
    // The IPC path: a `TemplateDocument` can reach `validate_template` without
    // ever passing the probe.
    let mut document = accepted(&sound());
    document.schema_version = 2;
    match validate_template(&document) {
        Err(TemplateError::UnsupportedSchemaVersion { found, supported }) => {
            assert_eq!(found, 2);
            assert_eq!(supported, SCHEMA_VERSION);
        }
        other => panic!("expected a version diagnostic, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Ranges, at the edge and one step past it
//
// Two values per bound, because only the pair distinguishes "checked" from
// "clamped" and from "not checked at all".
// ---------------------------------------------------------------------------

#[test]
fn a_position_at_either_end_of_the_bleed_is_accepted() {
    for (pointer, value) in [
        ("/safe_area/x", 1.5),
        ("/safe_area/x", -0.5),
        ("/safe_area/y", 1.5),
        ("/safe_area/y", -0.5),
        ("/layers/2/box/x", 1.5),
        ("/layers/2/box/y", -0.5),
        ("/layers/1/geometry/x", -0.5),
        ("/layers/1/geometry/y", 1.5),
    ] {
        accepted(&with(pointer, json!(value)));
    }
}

#[test]
fn a_position_one_step_past_the_bleed_is_rejected_and_not_pulled_back() {
    for (pointer, field, value) in [
        ("/safe_area/x", "safe_area.x", 1.51),
        ("/safe_area/x", "safe_area.x", -0.51),
        ("/safe_area/y", "safe_area.y", 1.51),
        ("/safe_area/y", "safe_area.y", -0.51),
        ("/layers/2/box/x", "layers[2].box.x", 1.51),
        ("/layers/2/box/y", "layers[2].box.y", -0.51),
        ("/layers/1/geometry/x", "layers[1].geometry.x", -0.51),
        ("/layers/1/geometry/y", "layers[1].geometry.y", 1.51),
    ] {
        let err = refused(&with(pointer, json!(value)));
        assert_out_of_range(&err, field);
        let TemplateError::OutOfRange {
            value: reported,
            min,
            max,
            ..
        } = &err
        else {
            unreachable!()
        };
        assert_eq!(*reported, value, "the diagnostic must quote what was found");
        assert_eq!(*min, -0.5);
        assert_eq!(*max, 1.5);
    }
}

#[test]
fn an_extent_may_be_zero_or_bleed_but_may_not_be_negative() {
    for pointer in ["/safe_area/w", "/safe_area/h", "/layers/2/box/w"] {
        accepted(&with(pointer, json!(0.0)));
        accepted(&with(pointer, json!(1.5)));
    }
    // -0.1 is inside `[-0.5, 1.5]`, so an implementation that bounded extents
    // like positions would accept all three of these.
    let err = refused(&with("/safe_area/w", json!(-0.1)));
    assert_out_of_range(&err, "safe_area.w");
    let err = refused(&with("/layers/2/box/h", json!(-0.1)));
    assert_out_of_range(&err, "layers[2].box.h");
    let err = refused(&with("/layers/1/geometry/w", json!(-0.1)));
    assert_out_of_range(&err, "layers[1].geometry.w");
    // And the far side.
    let err = refused(&with("/safe_area/w", json!(1.51)));
    assert_out_of_range(&err, "safe_area.w");
}

#[test]
fn an_opacity_is_a_fraction_of_one_at_both_ends() {
    accepted(&with("/layers/1/fill/opacity", json!(0.0)));
    accepted(&with("/layers/1/fill/opacity", json!(1.0)));
    let err = refused(&with("/layers/1/fill/opacity", json!(1.01)));
    assert_out_of_range(&err, "layers[1].fill.opacity");
    let err = refused(&with("/layers/1/fill/opacity", json!(-0.01)));
    assert_out_of_range(&err, "layers[1].fill.opacity");
}

#[test]
fn a_type_size_of_zero_is_rejected() {
    // Zero is inside the length span every other normalised length uses, so
    // this is a separate rule and needs its own case: FR-310 divides by it.
    let err = refused(&with("/layers/2/typography/size", json!(0.0)));
    assert_malformed(&err, "layers[2].typography.size");

    let mut document = with("/layers/2/typography/size", json!(0.05));
    document["layers"][2]["typography"]["min_size"] = json!(0.0);
    let err = refused(&document);
    assert_malformed(&err, "layers[2].typography.min_size");

    // And the smallest positive size is still a size.
    let mut document = with("/layers/2/typography/size", json!(0.001));
    document["layers"][2]["typography"]["min_size"] = json!(0.001);
    accepted(&document);
}

#[test]
fn a_shrink_floor_above_the_starting_size_is_rejected() {
    let mut document = sound();
    document["layers"][2]["typography"]["size"] = json!(0.05);
    document["layers"][2]["typography"]["min_size"] = json!(0.0500001);
    let err = refused(&document);
    assert_malformed(&err, "layers[2].typography.min_size");

    // Equal is the edge and is legal: a slot that never shrinks.
    let mut document = sound();
    document["layers"][2]["typography"]["size"] = json!(0.05);
    document["layers"][2]["typography"]["min_size"] = json!(0.05);
    accepted(&document);
}

/// `count` background layers, each with a distinct canonical UUID.
fn background_layers(count: usize) -> Vec<Value> {
    (0..count)
        .map(|index| {
            json!({
                "id": format!("0191aaaa-0000-7000-8000-0000000{index:05}"),
                "type": "background",
                "visible": true,
                "fill": { "kind": "solid", "color": "#0B0F1A" }
            })
        })
        .collect()
}

#[test]
fn the_layer_cap_is_thirty_two_and_the_thirty_third_is_refused() {
    assert_eq!(MAX_LAYERS, 32, "Appendix B says at most 32 layers");

    accepted(&with("/layers", json!(background_layers(MAX_LAYERS))));

    let err = refused(&with("/layers", json!(background_layers(MAX_LAYERS + 1))));
    let TemplateError::IntegerOutOfRange {
        field, value, max, ..
    } = &err
    else {
        panic!("expected a layer-count diagnostic, got {err:?}");
    };
    assert_eq!(field, "layers");
    assert_eq!(*value, 33);
    assert_eq!(*max, 32);
}

#[test]
fn a_document_of_exactly_the_size_limit_loads_and_one_byte_more_does_not() {
    assert_eq!(
        MAX_DOCUMENT_BYTES,
        256 * 1024,
        "Appendix B says under 256 KB"
    );

    let body = text(&sound());
    assert!(body.len() < MAX_DOCUMENT_BYTES, "the fixture must fit");

    // Padded with leading whitespace, which JSON ignores, so the only thing
    // that changes between the two cases is the byte count.
    let exactly = format!("{}{body}", " ".repeat(MAX_DOCUMENT_BYTES - body.len()));
    assert_eq!(exactly.len(), MAX_DOCUMENT_BYTES);
    parse_template(&exactly).expect("a document of exactly the limit is under the limit");

    let one_more = format!(" {exactly}");
    assert_eq!(one_more.len(), MAX_DOCUMENT_BYTES + 1);
    match parse_template(&one_more) {
        Err(TemplateError::TooLarge { bytes, limit }) => {
            assert_eq!(bytes, MAX_DOCUMENT_BYTES + 1);
            assert_eq!(limit, MAX_DOCUMENT_BYTES);
        }
        other => panic!("expected a size diagnostic, got {other:?}"),
    }
}

#[test]
fn a_not_a_number_is_refused_by_validate_template() {
    // Unreachable from JSON text — `serde_json` will not carry the literal
    // `NaN` — but perfectly reachable from a document built in code or handed
    // over IPC, which is the path `validate_template` exists to serve. This is
    // the case that distinguishes `value >= min && value <= max` from the
    // natural-looking `value < min || value > max`: every comparison against a
    // NaN is false, so the second spelling accepts one.
    let base = accepted(&sound());

    let mut document = base.clone();
    document.safe_area.x = f64::NAN;
    assert!(
        validate_template(&document).is_err(),
        "a NaN coordinate must not reach a renderer"
    );

    let mut document = base.clone();
    document.safe_area.w = f64::NAN;
    assert!(validate_template(&document).is_err(), "a NaN extent too");

    let mut document = base.clone();
    let Layer::Text {
        typography,
        effects,
        ..
    } = &mut document.layers[2]
    else {
        panic!("layer 2 of the fixture is a text layer");
    };
    typography.line_height = f64::NAN;
    effects.shadow.offset_x = f64::NAN;
    assert!(
        validate_template(&document).is_err(),
        "a NaN inside typography must not reach a renderer"
    );

    // The infinities go the same way.
    for bad in [f64::INFINITY, f64::NEG_INFINITY] {
        let mut document = base.clone();
        document.safe_area.y = bad;
        assert!(
            validate_template(&document).is_err(),
            "{bad} must be refused"
        );
    }
}

#[test]
fn json_text_cannot_carry_a_nan_or_an_overflowing_number() {
    // The companion to the test above: it says why a NaN cannot arrive through
    // `parse_template`, so a reader does not conclude the NaN check is dead.
    let body = text(&sound()).replace("\"x\":0.05", "\"x\":NaN");
    assert!(
        parse_template(&body).is_err(),
        "the NaN literal is not JSON"
    );

    let err = refused(&with("/safe_area/x", json!(1e308)));
    assert_out_of_range(&err, "safe_area.x");
}

// ---------------------------------------------------------------------------
// Strings: colours, ids, font names, path data
// ---------------------------------------------------------------------------

#[test]
fn a_colour_is_three_or_six_hex_digits_and_nothing_else() {
    for good in ["#FFF", "#FFFFFF", "#000", "#0B0F1A", "#abc", "#aBcDeF"] {
        accepted(&with("/layers/1/fill/color", json!(good)));
    }
    for bad in [
        "#FFFFFFFF",
        "#FFFF",
        "#FF",
        "#GGG",
        "#GGGGGG",
        "#0B0F1A;color:red",
        "red",
        "rgb(255,0,0)",
        "url(#x)",
        "var(--x)",
        "expression(alert(1))",
        "#",
        "",
        " #FFF",
        "#FFF ",
    ] {
        let err = refused(&with("/layers/1/fill/color", json!(bad)));
        assert_malformed(&err, "layers[1].fill.color");
    }
}

#[test]
fn every_colour_field_in_the_document_is_held_to_the_same_rule() {
    // One loosened call site is as bad as a loosened rule, so each is named.
    // `#GGG` starts with a `#`, which is what tells a real hex check apart
    // from a check that only looks at the first character.
    for pointer in [
        "/layers/1/fill/color",
        "/layers/1/stroke/color",
        "/layers/2/typography/color",
        "/layers/2/effects/shadow/color",
        "/layers/2/effects/outline/color",
    ] {
        assert!(
            parse_template(&text(&with(pointer, json!("url(#x)")))).is_err(),
            "{pointer} accepted a CSS url()"
        );
        assert!(
            parse_template(&text(&with(pointer, json!("#GGG")))).is_err(),
            "{pointer} accepted a non-hex colour that starts with #"
        );
        assert!(
            parse_template(&text(&with(pointer, json!("#FFFFFFFF")))).is_err(),
            "{pointer} accepted an eight-digit colour"
        );
    }
    let err = refused(&with("/layers/0/fill/color", json!("#GGG")));
    assert_malformed(&err, "layers[0].fill.color");

    let mut document = with("/layers/0/fill", gradient_fill());
    document["layers"][0]["fill"]["stops"][1]["color"] = json!("#GGG");
    let err = refused(&document);
    assert_malformed(&err, "layers[0].fill.stops[1].color");
}

#[test]
fn an_id_must_be_a_canonical_uuid() {
    for bad in [
        "background",
        "018f2c40-7c1e-7a3b-9f10-2b7c5d8e1a0",
        "018f2c40-7c1e-7a3b-9f10-2b7c5d8e1a011",
        "018f2c407c1e7a3b9f102b7c5d8e1a01",
        "018f2c40_7c1e_7a3b_9f10_2b7c5d8e1a01",
        "018f2c40-7c1e-7a3b-9f10-2b7c5d8e1g01",
        "",
        "../../etc/passwd",
        "<script>",
    ] {
        let err = refused(&with("/layers/1/id", json!(bad)));
        assert_malformed(&err, "layers[1].id");
    }
    // The letters are not case-significant, so both spellings are one id.
    accepted(&with(
        "/layers/1/id",
        json!("018F2C40-7C1E-7A3B-9F10-2B7C5D8E1A0B"),
    ));
}

#[test]
fn the_document_id_and_a_media_id_are_uuids_too() {
    let err = refused(&with("/id", json!("photo-with-scrim")));
    assert_malformed(&err, "id");

    let mut fill = image_fill();
    fill["media_id"] = json!("../../secret.png");
    let err = refused(&with("/layers/0/fill", fill));
    assert_malformed(&err, "layers[0].fill.media_id");
}

#[test]
fn two_layers_may_not_share_an_id_however_it_is_spelled() {
    let shared = "0191bbbb-0000-7000-8000-00000000000a";

    let mut document = sound();
    document["layers"][0]["id"] = json!(shared);
    document["layers"][1]["id"] = json!(shared);
    match refused(&document) {
        TemplateError::DuplicateLayerId { field } => assert_eq!(field, "layers[1]"),
        other => panic!("expected a duplicate-id diagnostic, got {other:?}"),
    }

    // Same id, different capitalisation. A case-sensitive check would let this
    // through and the collision would surface later, inside whichever consumer
    // normalises first.
    let mut document = sound();
    document["layers"][0]["id"] = json!(shared);
    document["layers"][2]["id"] = json!(shared.to_uppercase());
    match refused(&document) {
        TemplateError::DuplicateLayerId { field } => assert_eq!(field, "layers[2]"),
        other => panic!("expected a duplicate-id diagnostic, got {other:?}"),
    }

    // Three distinct ids stay distinct.
    accepted(&sound());
}

#[test]
fn a_font_family_may_not_carry_css_punctuation() {
    for good in ["Inter", "Segoe UI", "Noto Sans", "PT_Sans", "Source Sans 3"] {
        accepted(&with("/layers/2/typography/font_family", json!(good)));
    }
    for bad in [
        "Inter; color: red",
        "Inter;}",
        "url(https://example.invalid/f.woff2)",
        "Inter\", sans-serif",
        "'Inter'",
        "Inter, sans-serif",
        "Inter\\3c script",
        "",
        " Inter",
        "Inter ",
    ] {
        let err = refused(&with("/layers/2/typography/font_family", json!(bad)));
        assert_malformed(&err, "layers[2].typography.font_family");
    }
}

#[test]
fn a_fallback_family_is_held_to_the_same_rule_as_the_family() {
    let err = refused(&with(
        "/layers/2/typography/font_fallback",
        json!(["Segoe UI", "url(https://example.invalid/f.woff2)"]),
    ));
    assert_malformed(&err, "layers[2].typography.font_fallback[1]");
}

#[test]
fn svg_path_data_may_hold_only_commands_numbers_and_separators() {
    for good in [
        "M0,0 L1,0 L1,1 Z",
        "M 0 0 C 0.1 0.2, 0.3 0.4, 0.5 0.6 z",
        "M0,0 A0.5,0.5 0 1,0 1,1",
        "M1e-3,0 L1E2,0",
        "m0 0h1v1h-1z",
    ] {
        let mut geometry = path_geometry();
        geometry["d"] = json!(good);
        accepted(&with("/layers/1/geometry", geometry));
    }
    for bad in [
        "M0,0 <script>alert(1)</script>",
        "M0,0 \" onload=\"alert(1)",
        "M0,0 &#x3c;script&#x3e;",
        "M0,0 url(#x)",
        "M0,0 L1,0'",
        "M0,0 L1,0\\",
        "",
    ] {
        let mut geometry = path_geometry();
        geometry["d"] = json!(bad);
        let err = refused(&with("/layers/1/geometry", geometry));
        assert_malformed(&err, "layers[1].geometry.d");
    }
}

#[test]
fn a_viewbox_is_four_finite_numbers_with_a_positive_extent() {
    for good in ["0 0 1 1", "0,0,100,100", "-10 -10 20 20"] {
        let mut geometry = path_geometry();
        geometry["viewbox"] = json!(good);
        accepted(&with("/layers/1/geometry", geometry));
    }
    for bad in [
        "0 0 1",
        "0 0 1 1 1",
        "0 0 0 1",
        "0 0 1 0",
        "0 0 -1 1",
        "0 0 NaN 1",
        "0 0 inf 1",
        "<svg>",
        "",
    ] {
        let mut geometry = path_geometry();
        geometry["viewbox"] = json!(bad);
        let err = refused(&with("/layers/1/geometry", geometry));
        assert_malformed(&err, "layers[1].geometry.viewbox");
    }
}

#[test]
fn a_template_name_is_present_and_free_of_deceptive_characters() {
    let err = refused(&with("/name", json!("")));
    assert_malformed(&err, "name");
    let err = refused(&with("/name", json!("   ")));
    assert_malformed(&err, "name");
    // U+202E reverses everything after it in a picker.
    let err = refused(&with("/name", json!("Fixture \u{202e}gnp.xcod")));
    assert_malformed(&err, "name");
    let err = refused(&with("/name", json!("Fixture\u{0007}")));
    assert_malformed(&err, "name");
    // An author may be blank; a name may not.
    accepted(&with("/metadata/author", json!("")));
    let err = refused(&with("/metadata/author", json!("Author\u{202e}")));
    assert_malformed(&err, "metadata.author");
}

#[test]
fn a_timestamp_is_checked_for_shape_and_bounded_in_length() {
    accepted(&with("/metadata/created_at", json!("2026-08-07T10:00:00Z")));
    accepted(&with(
        "/metadata/created_at",
        json!("2026-08-07T10:00:00.123+07:00"),
    ));
    for bad in ["", "yesterday", "2026-08-07", "<script>alert(1)</script>"] {
        let err = refused(&with("/metadata/updated_at", json!(bad)));
        assert_malformed(&err, "metadata.updated_at");
    }
}

#[test]
fn the_counted_and_integer_settings_are_bounded_at_both_ends() {
    // A gradient needs two ends to be a gradient.
    let mut fill = gradient_fill();
    fill["stops"] = json!([{ "offset": 0.0, "color": "#000" }]);
    let err = refused(&with("/layers/0/fill", fill));
    match err {
        TemplateError::IntegerOutOfRange { field, value, .. } => {
            assert_eq!(field, "layers[0].fill.stops");
            assert_eq!(value, 1);
        }
        other => panic!("expected a stop-count diagnostic, got {other:?}"),
    }

    for (pointer, field, bad) in [
        (
            "/layers/2/typography/weight",
            "layers[2].typography.weight",
            0u64,
        ),
        (
            "/layers/2/typography/weight",
            "layers[2].typography.weight",
            1001,
        ),
        (
            "/layers/2/typography/max_lines",
            "layers[2].typography.max_lines",
            0,
        ),
        (
            "/layers/2/typography/max_lines",
            "layers[2].typography.max_lines",
            65,
        ),
        ("/canvas/reference_width", "canvas.reference_width", 0),
        ("/canvas/reference_height", "canvas.reference_height", 0),
    ] {
        let err = refused(&with(pointer, json!(bad)));
        match err {
            TemplateError::IntegerOutOfRange { field: named, .. } => assert_eq!(named, field),
            other => panic!("expected {field} to be out of range, got {other:?}"),
        }
    }

    // And the values just inside each edge are accepted.
    accepted(&with("/layers/2/typography/weight", json!(1)));
    accepted(&with("/layers/2/typography/weight", json!(1000)));
    accepted(&with("/layers/2/typography/max_lines", json!(1)));
    accepted(&with("/layers/2/typography/max_lines", json!(64)));
}

// ---------------------------------------------------------------------------
// The shape of the input itself
// ---------------------------------------------------------------------------

#[test]
fn text_that_is_not_a_template_document_is_a_diagnostic_and_not_a_panic() {
    for bad in [
        "",
        "   ",
        "null",
        "[]",
        "42",
        "{",
        "{\"schema_version\": 1",
        "<svg/>",
    ] {
        assert!(
            parse_template(bad).is_err(),
            "{bad:?} must be refused, not accepted"
        );
    }
}

#[test]
fn a_missing_required_field_names_itself() {
    let mut document = sound();
    document["layers"][2]["typography"]
        .as_object_mut()
        .unwrap()
        .remove("min_size");
    let err = refused(&document);
    assert_syntax(&err, "min_size");
}

// ---------------------------------------------------------------------------
// Display text: what `name` and `metadata.author` may and may not carry
//
// `check_display_text` guards the two strings an operator reads in the picker
// (FR-408), and it is the one place in this module where the rule is *not*
// "an allowlist of the characters the field is made of" — human text is made
// of anything. So it is a subtraction, and each group it subtracts is written
// to fail a specific wrong implementation:
//
// - a hand-listed set of bidi controls instead of the whole of `Cf`, which
//   leaves out U+061C ARABIC LETTER MARK;
// - `char::is_control` alone, which walks past U+2028 and U+2029;
// - `models::text::is_format_char` alone — that is, sharing one predicate with
//   `scripture::is_invisible` — which walks past the separators *and* the four
//   Hangul fillers;
// - dropping the path-separator rule, or narrowing `..` to a prefix test.
//
// And one group it deliberately does *not* subtract: the markup
// metacharacters. See `markup_metacharacters_in_a_name_are_accepted_on_purpose`
// for why that is a decision and not an oversight.
// ---------------------------------------------------------------------------

/// Asserts the refusal named a field as malformed, and gave the reason written
/// for that kind of fault rather than another field's.
#[track_caller]
fn assert_malformed_because(err: &TemplateError, field: &str, needle: &str) {
    let TemplateError::Malformed {
        field: named,
        reason,
    } = err
    else {
        panic!("expected {field} to be malformed, got {err:?}");
    };
    assert_eq!(named, field, "the wrong field was named");
    assert!(
        reason.contains(needle),
        "expected the reason for {field} to mention {needle:?}, got {reason:?}"
    );
}

/// Every invisible character refused in a name and in an author, named one at
/// a time.
///
/// U+061C is why this is a *category* rule and not a list. It does exactly
/// what U+200E LRM and U+200F RLM do, and every hand-written "the bidi
/// controls" set leaves it out, because it was added to Unicode after the list
/// everyone copies from was written. Taking the whole of `Cf` — the table
/// `models::text` shares with `scripture` — is what keeps it in.
///
/// U+2028 and U+2029 are the other half of the pairing: they are `Zl` and
/// `Zp`, so `char::is_control` answers `false` for both, and a check spelled
/// only as `is_control() || is_format_char()` would let a name break a line in
/// the middle of the picker.
#[test]
fn every_invisible_character_is_refused_in_a_name_and_in_an_author() {
    for c in [
        '\u{061c}', // ARABIC LETTER MARK — the one a hand-listed bidi set omits
        '\u{200b}', // ZERO WIDTH SPACE
        '\u{200c}', // ZERO WIDTH NON-JOINER
        '\u{200d}', // ZERO WIDTH JOINER
        '\u{200e}', // LEFT-TO-RIGHT MARK
        '\u{200f}', // RIGHT-TO-LEFT MARK
        '\u{202a}', // LEFT-TO-RIGHT EMBEDDING
        '\u{202e}', // RIGHT-TO-LEFT OVERRIDE
        '\u{2060}', // WORD JOINER
        '\u{2066}', // LEFT-TO-RIGHT ISOLATE
        '\u{00ad}', // SOFT HYPHEN
        '\u{180e}', // MONGOLIAN VOWEL SEPARATOR
        '\u{feff}', // ZERO WIDTH NO-BREAK SPACE
        '\u{2028}', // LINE SEPARATOR — `Zl`, so not a control character
        '\u{2029}', // PARAGRAPH SEPARATOR — `Zp`, likewise
        '\u{0009}', // TAB — `Cc`, and refused here though `scripture` keeps it
        '\u{0085}', // NEL — `Cc`, and invisible in a single-line picker row
        '\u{007f}', // DELETE
    ] {
        let err = refused(&with("/name", json!(format!("Fixture{c}Template"))));
        assert_malformed_because(&err, "name", "invisible");

        let err = refused(&with("/metadata/author", json!(format!("Author{c}"))));
        assert_malformed_because(&err, "metadata.author", "invisible");
    }
}

/// `<`, `>`, `&`, `"` and `'` in a name or an author are **accepted, and that
/// is a decision** — not a gap this test is waiting for someone to close.
///
/// A template name is human text. "Natal & Tahun Baru" is an ordinary name for
/// a Christmas set, an apostrophe appears in a great many names, and quotation
/// marks around a section title are normal punctuation. A validator that
/// argued with its operator about any of them would be refusing correct input.
///
/// The defence against markup is escaping **at the render boundary**, and it
/// has to be, because this function cannot know what its consumer interpolates
/// the string into: an escape correct for HTML is wrong for a terminal and
/// wrong again for a filename. Adding `<` to the refused list here would look
/// like hardening and would buy nothing — a consumer that concatenates HTML is
/// still broken for every other input — while costing a name people actually
/// type. That obligation is recorded in the module header's table of what the
/// caller owns.
///
/// So this test exists to make that reasoning fail loudly if it is ever
/// quietly reversed. If you are here because this broke after you added a
/// metacharacter to the refused list: the change you want is at the consumer,
/// not here.
#[test]
fn markup_metacharacters_in_a_name_are_accepted_on_purpose() {
    for good in [
        "Natal & Tahun Baru",
        "<Malam> \"Kudus\"",
        "Perayaan O'Brien",
        "Ibadah 'Pagi'",
        "A & B <C> \"D\" 'E'",
    ] {
        let document = accepted(&with("/name", json!(good)));
        assert_eq!(
            document.name, good,
            "the name must survive validation exactly as it was written"
        );

        let document = accepted(&with("/metadata/author", json!(good)));
        assert_eq!(document.metadata.author, good);
    }
}

/// A path separator is refused in a name and in an author, because `name` is
/// the natural source of an `.aerotpl` export filename (FR-409).
///
/// `..` is refused **anywhere in the string**, not only at the front. That
/// costs a real name — "Natal.. Baru", an ellipsis typed with two stops — and
/// the cost is accepted: a traversal segment is a traversal segment wherever a
/// consumer splits the string, and no consumer has to think about it if it
/// cannot be there at all. A single `.` stays legal, so the rule is about the
/// pair and not about the character.
#[test]
fn a_path_separator_is_refused_in_a_name_and_in_an_author() {
    for good in [
        "Fixture Template 2026",
        "Natal. Baru",
        "v1.2.3 Fixture",
        "Fixture - Template_2",
    ] {
        accepted(&with("/name", json!(good)));
        accepted(&with("/metadata/author", json!(good)));
    }

    for bad in [
        "Fixture/Template",
        "Fixture\\Template",
        "C:Fixture",
        "..",
        "../Fixture",
        "Fixture..",
        "Fixture..Template",
        // The conscious consequence of refusing `..` anywhere: a name a person
        // could plausibly type is refused, and that is the trade this rule
        // makes rather than an accident of it.
        "Natal.. Baru",
        "/",
        "\\",
        ":",
    ] {
        let err = refused(&with("/name", json!(bad)));
        assert_malformed_because(&err, "name", "path separator");

        let err = refused(&with("/metadata/author", json!(bad)));
        assert_malformed_because(&err, "metadata.author", "path separator");
    }
}

/// The four Hangul fillers, at both places `is_invisible` is consulted.
///
/// These are the group a reader is most likely to delete as superstition, so
/// the test asserts the property that makes them dangerous before it asserts
/// the refusal: `char::is_alphanumeric` calls each of them a *letter*, and
/// every font draws nothing. That combination means a name made only of
/// fillers passes "must not be blank" — `str::trim` does not touch them — and
/// is still an empty row in the picker, and that `Inter\u{3164}` passes the
/// font-family allowlist and is indistinguishable from `Inter` in the Builder.
///
/// Refusing them as *characters* rather than folding them into the blank rule
/// is what makes one mechanism cover both call sites; `check_font_family` has
/// no blank rule to fold them into.
#[test]
fn the_hangul_fillers_are_refused_as_a_name_and_inside_a_font_family() {
    const FILLERS: [char; 4] = [
        '\u{115f}', // HANGUL CHOSEONG FILLER
        '\u{1160}', // HANGUL JUNGSEONG FILLER
        '\u{3164}', // HANGUL FILLER
        '\u{ffa0}', // HALFWIDTH HANGUL FILLER
    ];

    // The baseline, so every case below turns on the filler and not on the
    // rest of the string.
    accepted(&with("/layers/2/typography/font_family", json!("Inter")));

    for c in FILLERS {
        assert!(
            c.is_alphanumeric(),
            "U+{:04X} is a letter to `char::is_alphanumeric`, which is why the \
             font-family allowlist cannot exclude it on its own",
            c as u32
        );
        let only_filler = c.to_string();
        assert!(
            !only_filler.trim().is_empty(),
            "U+{:04X} survives `trim`, which is why the blank rule cannot \
             exclude it on its own",
            c as u32
        );

        let err = refused(&with("/name", json!(only_filler)));
        assert_malformed_because(&err, "name", "invisible");

        let err = refused(&with("/name", json!(format!("Fixture{c}"))));
        assert_malformed_because(&err, "name", "invisible");

        let err = refused(&with("/metadata/author", json!(format!("Author{c}"))));
        assert_malformed_because(&err, "metadata.author", "invisible");

        let err = refused(&with(
            "/layers/2/typography/font_family",
            json!(format!("Inter{c}")),
        ));
        assert_malformed(&err, "layers[2].typography.font_family");

        let err = refused(&with(
            "/layers/2/typography/font_fallback",
            json!(["Segoe UI", format!("Inter{c}")]),
        ));
        assert_malformed(&err, "layers[2].typography.font_fallback[1]");
    }
}

/// Unicode general categories `Cc` and `Cf`, as inclusive code point ranges.
///
/// The same 23 ranges `tests/scripture_reference.rs` holds, written out a
/// second time on purpose rather than shared. `models::text::is_format_char`
/// now has two callers, and a table checked from only one of them is a table
/// the other caller can be refactored away from without anything going red.
/// See the scripture file for where the ranges came from: two independent
/// implementations of the UCD, cross-checked, and neither of them a dependency
/// of this project (NFR-16).
const CONTROL_AND_FORMAT_RANGES: [(u32, u32); 23] = [
    (0x0000, 0x001f),
    (0x007f, 0x009f),
    (0x00ad, 0x00ad),
    (0x0600, 0x0605),
    (0x061c, 0x061c),
    (0x06dd, 0x06dd),
    (0x070f, 0x070f),
    (0x0890, 0x0891),
    (0x08e2, 0x08e2),
    (0x180e, 0x180e),
    (0x200b, 0x200f),
    (0x202a, 0x202e),
    (0x2060, 0x2064),
    (0x2066, 0x206f),
    (0xfeff, 0xfeff),
    (0xfff9, 0xfffb),
    (0x110bd, 0x110bd),
    (0x110cd, 0x110cd),
    (0x13430, 0x1343f),
    (0x1bca0, 0x1bca3),
    (0x1d173, 0x1d17a),
    (0xe0001, 0xe0001),
    (0xe0020, 0xe007f),
];

/// The code points `check_display_text` refuses that are **neither `Cc` nor
/// `Cf`** — that is, precisely the difference between this module's
/// `is_invisible` and `scripture::is_invisible`.
///
/// Written as a separate list so the difference is a value the test can assert
/// about, rather than a sentence in a comment. The exhaustive walk below
/// checks each of these really is outside [`CONTROL_AND_FORMAT_RANGES`]; if
/// one ever moves inside, the two predicates have converged and this list
/// should shrink rather than the assertion loosen.
const REFUSED_BUT_NOT_CONTROL_OR_FORMAT: [u32; 6] = [
    0x2028, // LINE SEPARATOR, `Zl`
    0x2029, // PARAGRAPH SEPARATOR, `Zp`
    0x115f, // HANGUL CHOSEONG FILLER, `Lo`
    0x1160, // HANGUL JUNGSEONG FILLER, `Lo`
    0x3164, // HANGUL FILLER, `Lo`
    0xffa0, // HALFWIDTH HANGUL FILLER, `Lo`
];

/// The path separators, which are ordinary printable ASCII and so belong to no
/// Unicode category this rule could have reached for.
const PATH_SEPARATORS: [char; 3] = ['/', '\\', ':'];

fn is_control_or_format(code_point: u32) -> bool {
    CONTROL_AND_FORMAT_RANGES
        .iter()
        .any(|(first, last)| (*first..=*last).contains(&code_point))
}

/// FR-401 / NFR-28 — every Unicode code point, checked against the UCD rather
/// than against the implementation's own table.
///
/// The scripture side of `models::text` is already covered exhaustively by
/// `exactly_the_control_and_format_characters_are_deleted`; this is the same
/// walk from the other caller, so the shared `Cf` table is pinned from both
/// sides and cannot be widened or narrowed for one module's benefit.
///
/// **The difference from `scripture::is_invisible` is stated, not smoothed
/// over.** `check_display_text` refuses strictly more: the two `Zl`/`Zp`
/// separators, the four Hangul fillers, and the three path separators. Each is
/// enumerated above, and the first assertion in the body proves the six
/// non-ASCII ones lie outside `Cc`/`Cf` — so a refactor that replaced this
/// module's predicate with `scripture`'s would fail here naming the code point
/// it started accepting, which is exactly the merge the doc comment on
/// `is_invisible` warns against.
///
/// Every candidate is prefixed with a word, because what is under test is the
/// character and not the blank rule.
#[test]
fn exactly_the_invisible_characters_and_path_separators_are_refused_in_a_name() {
    for code_point in REFUSED_BUT_NOT_CONTROL_OR_FORMAT {
        assert!(
            !is_control_or_format(code_point),
            "U+{code_point:04X} is listed as a refusal this module makes and \
             `scripture` does not, but the UCD calls it Cc or Cf — the two \
             predicates have converged and this list is stale"
        );
    }
    for separator in PATH_SEPARATORS {
        assert!(
            !is_control_or_format(separator as u32),
            "{separator:?} is refused as a path separator, not as an invisible"
        );
    }

    // No layers and no author, so the walk pays for one `name` check and not
    // for a document: 1 114 112 iterations is worth trimming.
    let mut document = accepted(&with("/layers", json!([])));
    document.metadata.author = String::new();

    for code_point in 0..=0x10_ffffu32 {
        let Some(c) = char::from_u32(code_point) else {
            continue; // a surrogate, which is not a character
        };

        document.name.clear();
        document.name.push_str("Fixture");
        document.name.push(c);

        let extra =
            REFUSED_BUT_NOT_CONTROL_OR_FORMAT.contains(&code_point) || PATH_SEPARATORS.contains(&c);
        let expected_refused = is_control_or_format(code_point) || extra;
        let was_refused = validate_template(&document).is_err();

        assert_eq!(
            was_refused,
            expected_refused,
            "U+{code_point:04X}: refused={was_refused}, but the UCD says \
             Cc/Cf={} and this module's extra refusals say {extra}",
            is_control_or_format(code_point)
        );
    }
}
