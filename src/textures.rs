use std::error::Error;
use std::fmt;

use bevy::prelude::*;
use bevy::render::once_cell::sync::OnceCell;
use bevy::utils::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureMapId(usize);

static MAP: OnceCell<TextureMap> = OnceCell::new();

/// The combined texture atlas for all of the blocks.
#[derive(Debug)]
pub struct TextureMap {
    atlas: TextureAtlas,
    mapping: HashMap<String, TextureMapId>,
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

impl TextureMap {
    /// Build the texture atlas of the list of texture `handles`.
    pub fn build(
        handles: &[Handle<Image>],
        asset_server: &AssetServer,
        images: &mut Assets<Image>,
    ) -> Result<(), anyhow::Error> {
        let mut atlas = TextureAtlasBuilder::default();

        for handle in handles {
            let image = images.get(handle).ok_or(TextureMapError)?;
            atlas.add_texture(handle.clone_weak(), image);
        }

        let atlas = atlas.finish(images)?;

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
                TextureMapId(atlas.get_texture_index(&handle).unwrap()),
            );
        }

        MAP.set(TextureMap { atlas, mapping }).unwrap();

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
    pub fn uv(&self, id: TextureMapId) -> (Vec2, Vec2) {
        assert!(id.0 < self.atlas.len());
        let rect = self.atlas.textures[id.0];
        let size = self.atlas.size;
        (rect.min / size, rect.max / size)
    }

    /// Return the numerical id for the given texture `name`.
    pub fn id(&self, ident: &str) -> TextureMapId {
        self.mapping[ident]
    }
}
