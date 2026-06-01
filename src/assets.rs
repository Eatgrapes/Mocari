use std::{fmt, fs, path::Path};

use crate::{
    json::Model3,
    moc3::{
        Moc3ArtMeshKeyforms, Moc3ArtMeshes, Moc3CanvasInfo, Moc3Deformers, Moc3DrawableMesh,
        Moc3KeyformBindings, build_moc3_drawable_meshes_for_default_pose,
    },
};


#[derive(Debug, Clone)]
pub struct DefaultModel {
    model: Model3,
    canvas: Moc3CanvasInfo,
    meshes: Vec<Moc3DrawableMesh>,
    textures: Vec<DecodedTexture>,
}

impl DefaultModel {
    pub fn model(&self) -> &Model3 {
        &self.model
    }

    pub fn canvas(&self) -> Moc3CanvasInfo {
        self.canvas
    }

    pub fn meshes(&self) -> &[Moc3DrawableMesh] {
        &self.meshes
    }

    pub fn textures(&self) -> &[DecodedTexture] {
        &self.textures
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedTexture {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl DecodedTexture {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba,
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

#[derive(Debug)]
pub enum AssetLoadError {
    Io {
        path: String,
        source: std::io::Error,
    },
    Json(crate::Error),
    Moc3(crate::Error),
    Image {
        path: String,
        source: image::ImageError,
    },
    MissingParent {
        path: String,
    },
    DrawableMeshes,
}

impl fmt::Display for AssetLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(formatter, "failed to read {path}: {source}"),
            Self::Json(error) => write!(formatter, "failed to parse model json: {error}"),
            Self::Moc3(error) => write!(formatter, "failed to parse moc3: {error}"),
            Self::Image { path, source } => write!(formatter, "failed to decode {path}: {source}"),
            Self::MissingParent { path } => write!(formatter, "path has no parent: {path}"),
            Self::DrawableMeshes => formatter.write_str("failed to build drawable meshes"),
        }
    }
}

impl std::error::Error for AssetLoadError {}

pub fn load_model(path: impl AsRef<Path>) -> Result<DefaultModel, AssetLoadError> {
    let path = path.as_ref();
    let model_source = read_text(path)?;
    let model = Model3::from_json_str(&model_source).map_err(AssetLoadError::Json)?;
    let model_dir = path.parent().ok_or_else(|| AssetLoadError::MissingParent {
        path: path.display().to_string(),
    })?;
    let moc_path = model_dir.join(model.moc());
    let moc = read_bytes(&moc_path)?;

    let art_meshes = Moc3ArtMeshes::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let keyforms = Moc3ArtMeshKeyforms::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let deformers = Moc3Deformers::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let bindings = Moc3KeyformBindings::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let canvas = Moc3CanvasInfo::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let meshes =
        build_moc3_drawable_meshes_for_default_pose(&art_meshes, &keyforms, &deformers, &bindings)
            .ok_or(AssetLoadError::DrawableMeshes)?;
    let textures = model
        .textures()
        .iter()
        .map(|texture| decode_texture(model_dir.join(texture)))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(DefaultModel {
        model,
        canvas,
        meshes,
        textures,
    })
}

fn read_text(path: &Path) -> Result<String, AssetLoadError> {
    fs::read_to_string(path).map_err(|source| AssetLoadError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, AssetLoadError> {
    fs::read(path).map_err(|source| AssetLoadError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn decode_texture(path: impl AsRef<Path>) -> Result<DecodedTexture, AssetLoadError> {
    let path = path.as_ref();
    let image = image::open(path).map_err(|source| AssetLoadError::Image {
        path: path.display().to_string(),
        source,
    })?;
    let rgba = image.to_rgba8();
    Ok(DecodedTexture::new(
        rgba.width(),
        rgba.height(),
        rgba.into_raw(),
    ))
}
