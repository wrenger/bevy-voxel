use std::error::Error;
use std::fmt;
use std::sync::OnceLock;

use bevy::prelude::*;
use bevy::render::texture::ImageSampler;
use bevy::utils::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TileTextureId(usize);

static MAP: OnceLock<TileTextures> = OnceLock::new();

/// The combined texture atlas for all of the blocks.
#[derive(Debug)]
pub struct TileTextures {
    atlas: TextureAtlas,
    mapping: HashMap<String, TileTextureId>,
}

/// Error during texture atlas generation.
#[derive(Debug)]
struct TextureMapError;

impl Error for TextureMapError {}

impl fmt::Display for TextureMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Error parsing textures")
    }
}

impl TileTextures {
    /// Build the texture atlas of the list of texture `handles`.
    pub fn build(
        handles: &[Handle<Image>],
        asset_server: &AssetServer,
        images: &mut Assets<Image>,
    ) -> Result<(), anyhow::Error> {
        let mut atlas = TextureAtlasBuilder::default();

        for handle in handles {
            let image = images.get_mut(handle).ok_or(TextureMapError)?;
            atlas.add_texture(handle.clone_weak(), image);
        }

        let atlas = atlas.finish(images)?;

        // Texture filtering
        let image = images.get_mut(&atlas.texture).unwrap();
        image.sampler_descriptor = ImageSampler::nearest();

        let mut mapping = HashMap::new();
        for handle in handles {
            let path = asset_server
                .get_handle_path(handle)
                .ok_or(TextureMapError)?;
            let name = path
                .path()
                .file_stem()
                .ok_or(TextureMapError)?
                .to_string_lossy();
            mapping.insert(
                name.into_owned(),
                TileTextureId(atlas.get_texture_index(handle).unwrap()),
            );
        }

        MAP.set(TileTextures { atlas, mapping }).unwrap();

        Ok(())
    }

    pub fn get<'a>() -> &'a Self {
        MAP.get().as_ref().expect("Textures not initialized")
    }

    /// Return the combined texture image.
    pub fn image(&self) -> Handle<Image> {
        self.atlas.texture.clone()
    }

    /// Return the uv coordinates for the given texture `id`.
    pub fn uv(&self, id: TileTextureId) -> (Vec2, Vec2) {
        const V2_EPS: f32 = 0.0001;

        assert!(id.0 < self.atlas.len());
        let rect = self.atlas.textures[id.0];
        let size = self.atlas.size;
        (rect.min / size + V2_EPS, rect.max / size - V2_EPS)
    }

    /// Return the numerical id for the given texture `name`.
    pub fn id(&self, ident: &str) -> TileTextureId {
        self.mapping[ident]
    }
}
