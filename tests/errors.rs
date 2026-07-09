use std::error::Error as StdError;

use mocari::{
    Error, assets::AssetLoadError, expression::ExpressionLoadError, motion::MotionLoadError,
};

#[test]
fn load_errors_expose_underlying_sources() {
    let parser_error = Error::InvalidJson {
        format: "motion3.json",
        message: "expected object".to_string(),
    };
    let motion_error = MotionLoadError::Parse(parser_error.clone());
    assert_eq!(
        motion_error.to_string(),
        "failed to parse motion3: invalid motion3.json: expected object"
    );
    assert_eq!(
        motion_error.source().map(ToString::to_string),
        Some("invalid motion3.json: expected object".to_string())
    );

    let expression_error = ExpressionLoadError::Parse(parser_error.clone());
    assert_eq!(
        expression_error.source().map(ToString::to_string),
        Some("invalid motion3.json: expected object".to_string())
    );

    let asset_json_error = AssetLoadError::Json(parser_error.clone());
    assert_eq!(
        asset_json_error.source().map(ToString::to_string),
        Some("invalid motion3.json: expected object".to_string())
    );

    let asset_moc_error = AssetLoadError::Moc3(Error::InvalidMoc3 {
        message: "bad offset".to_string(),
    });
    assert_eq!(
        asset_moc_error.source().map(ToString::to_string),
        Some("invalid moc3: bad offset".to_string())
    );
}

#[test]
fn io_load_errors_expose_underlying_sources() {
    let motion_error = MotionLoadError::Io {
        path: "motions/idle.motion3.json".to_string(),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing motion"),
    };
    assert_eq!(
        motion_error.to_string(),
        "failed to read motions/idle.motion3.json: missing motion"
    );
    assert_eq!(
        motion_error.source().map(ToString::to_string),
        Some("missing motion".to_string())
    );

    let expression_error = ExpressionLoadError::Io {
        path: "expressions/smile.exp3.json".to_string(),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing expression"),
    };
    assert_eq!(
        expression_error.source().map(ToString::to_string),
        Some("missing expression".to_string())
    );

    let asset_error = AssetLoadError::Io {
        path: "models/Hiyori/Hiyori.model3.json".to_string(),
        source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing model"),
    };
    assert_eq!(
        asset_error.source().map(ToString::to_string),
        Some("missing model".to_string())
    );
}
