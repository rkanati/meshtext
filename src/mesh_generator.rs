use std::collections::HashMap;

use glam::{Mat3, Mat4, Vec2, Vec3, Vec3A};
use ttf_parser::{Face, GlyphId};

use crate::{
    error::MeshTextError,
    util::{
        mesh_to_flat_2d, mesh_to_indexed_flat_2d, raster_to_mesh, raster_to_mesh_indexed,
        text_mesh_from_data, text_mesh_from_data_2d, text_mesh_from_data_indexed,
        text_mesh_from_data_indexed_2d, GlyphOutlineBuilder,
    },
    BoundingBox, Glyph, IndexedMeshText, MeshText, QualitySettings,
};

type Mesh = (Vec<Vec3A>, BoundingBox);
type Mesh2D = (Vec<Vec2>, BoundingBox);

type IndexedMesh = (Vec<u32>, Vec<Vec3A>, BoundingBox);
type IndexedMesh2D = (Vec<u32>, Vec<Vec2>, BoundingBox);

/// A [MeshGenerator] handles rasterizing individual glyphs.
///
/// Each [MeshGenerator] will handle exactly one font. This means
/// if you need support for multiple fonts, you will need to create
/// multiple instances (one per font) of this generator.
pub struct MeshGenerator {
    /// The current [Face].
    font: Face<'static>,

    /// Quality settings for generating the text meshes.
    quality: QualitySettings,
}

impl MeshGenerator {
    /// Creates a new [MeshGenerator].
    ///
    /// Arguments:
    ///
    /// * `font`: The font that will be used for rasterizing.
    pub fn new(font: &'static [u8]) -> Self {
        let face = Face::parse(font, 0).expect("Failed to generate font from data.");

        Self {
            font: face,
            quality: QualitySettings::default(),
        }
    }

    /// Creates a new [MeshGenerator] with custom quality settings.
    ///
    /// Arguments:
    ///
    /// * `font`: The font that will be used for rasterizing.
    /// * `quality`: The [QualitySettings] that should be used.
    pub fn new_with_quality(font: &'static [u8], quality: QualitySettings) -> Self {
        let face = Face::parse(font, 0).expect("Failed to generate font from data.");

        Self {
            font: face,
            quality,
        }
    }

    /*
    /// Fills the internal cache of a [MeshGenerator] with the given characters.
    ///
    /// Arguments:
    ///
    /// * `glyphs`: The glyphs that will be precached. Each character should appear exactly once.
    /// * `flat`: Wether the flat or three-dimensional variant of the characters should be preloaded.
    /// If both variants should be precached this function must be called twice with this parameter set
    /// to `true` and `false`.
    /// * `cache`: An optional value that controls which cache will be filled. `None` means both caches will be filled.
    ///
    /// Returns:
    ///
    /// A [Result] indicating if the operation was successful.
    ///
    /// # Example
    ///
    /// ```rust
    /// use meshtext::MeshGenerator;
    ///
    /// let font_data = include_bytes!("../assets/font/FiraMono-Regular.ttf");
    /// let mut generator = MeshGenerator::new(font_data);
    ///
    /// let common = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".to_string();
    ///
    /// // Precache both flat and three-dimensional glyphs both for indexed and non-indexed meshes.
    /// generator.precache_glyphs(&common, false, None);
    /// generator.precache_glyphs(&common, true, None);
    /// ```
    pub fn precache_glyphs(
        &mut self,
        glyphs: &str,
        flat: bool,
        cache: Option<CacheType>,
    ) -> Result<(), Box<dyn MeshTextError>> {
        if let Some(cache_type) = cache {
            match cache_type {
                CacheType::Normal => {
                    for c in glyphs.chars() {
                        self.generate_glyph(c, flat, None)?;
                    }
                }
                CacheType::Indexed => {
                    for c in glyphs.chars() {
                        self.generate_glyph_indexed(c, flat, None)?;
                    }
                }
            }
        } else {
            // If no type is set explicitely, both variants will be precached.
            for c in glyphs.chars() {
                self.generate_glyph(c, flat, None)?;
            }
            for c in glyphs.chars() {
                self.generate_glyph_indexed(c, flat, None)?;
            }
        }

        Ok(())
    }
    */

    /// Generates the [MeshText] of a single character with a custom transformation.
    ///
    /// Arguments:
    ///
    /// * `glyph`: The character that should be converted to a mesh.
    /// * `flat`: Set this to `true` for 2D meshes, or to `false` in order
    /// to generate a mesh with a depth of `1.0` units.
    /// * `transform`: The 4x4 homogenous transformation matrix in column
    /// major order that will be applied to this text.
    ///
    /// Returns:
    ///
    /// The desired [MeshText] or an [MeshTextError] if anything went wrong in the
    /// process.
    fn generate_glyph(
        &mut self,
        glyph: char,
        flat: bool,
        transform: Option<&[f32; 16]>,
    ) -> Result<MeshText, Box<dyn MeshTextError>> {
        let mut mesh = self.make_mesh(glyph, flat)?;

        if let Some(value) = transform {
            let transform = Mat4::from_cols_array(value);

            for v in mesh.0.iter_mut() {
                *v = transform.transform_point3a(*v);
            }
            mesh.1.transform(&transform);
        }

        Ok(text_mesh_from_data(mesh))
    }

    /// Generates the two-dimensional [MeshText] of a single character with a custom transformation.
    ///
    /// Arguments:
    ///
    /// * `glyph`: The character that should be converted to a mesh.
    /// * `transform`: The 3x3 homogenous transformation matrix in column
    /// major order that will be applied to this text.
    ///
    /// Returns:
    ///
    /// The desired two-dimensional [MeshText] or an [MeshTextError] if anything went wrong in the
    /// process.
    fn generate_glyph_2d(
        &mut self,
        glyph: char,
        transform: Option<&[f32; 9]>,
    ) -> Result<MeshText, Box<dyn MeshTextError>> {
        let mesh = self.make_mesh(glyph, true)?;
        let mut mesh = mesh_to_flat_2d(mesh);

        if let Some(value) = transform {
            let transform = Mat3::from_cols_array(value);

            for v in mesh.0.iter_mut() {
                *v = transform.transform_point2(*v);
            }
            mesh.1.transform_2d(&transform);
        }

        Ok(text_mesh_from_data_2d(mesh))
    }

    /// Generates the [IndexedMeshText] of a single character with a custom transformation.
    ///
    /// This function generates a mesh with indices and vertices.
    ///
    /// Arguments:
    ///
    /// * `glyph`: The character that should be converted to a mesh.
    /// * `flat`: Set this to `true` for 2D meshes, or to `false` in order
    /// to generate a mesh with a depth of `1.0` units.
    /// * `transform`: The 4x4 homogenous transformation matrix in column
    /// major order that will be applied to this text.
    ///
    /// Returns:
    ///
    /// The desired [IndexedMeshText] or an [MeshTextError] if anything went wrong in the
    /// process.
    fn generate_glyph_indexed(
        &mut self,
        glyph: char,
        flat: bool,
        transform: Option<&[f32; 16]>,
    ) -> Result<IndexedMeshText, Box<dyn MeshTextError>> {
        let mut mesh = self.make_indexed_mesh(glyph, flat)?;

        if let Some(value) = transform {
            let transform = Mat4::from_cols_array(value);

            for v in mesh.1.iter_mut() {
                *v = transform.transform_point3a(*v);
            }
            mesh.2.transform(&transform);
        }

        Ok(text_mesh_from_data_indexed(mesh))
    }

    /// Generates the two-dimensional [IndexedMeshText] of a single character
    /// with a custom transformation.
    ///
    /// This function generates a mesh with indices and vertices.
    ///
    /// Arguments:
    ///
    /// * `glyph`: The character that should be converted to a mesh.
    /// * `transform`: The 3x3 homogenous transformation matrix in column
    /// major order that will be applied to this text.
    ///
    /// Returns:
    ///
    /// The desired [IndexedMeshText] or an [MeshTextError] if anything went wrong in the
    /// process.
    fn generate_glyph_indexed_2d(
        &mut self,
        glyph: char,
        transform: Option<&[f32; 9]>,
    ) -> Result<IndexedMeshText, Box<dyn MeshTextError>> {
        let mesh = self.make_indexed_mesh(glyph, true)?;
        let mut mesh = mesh_to_indexed_flat_2d(mesh);

        if let Some(value) = transform {
            let transform = Mat3::from_cols_array(value);

            for v in mesh.1.iter_mut() {
                *v = transform.transform_point2(*v);
            }
            mesh.2.transform_2d(&transform);
        }

        Ok(text_mesh_from_data_indexed_2d(mesh))
    }

    /// Generates the [Mesh] of a single character with a custom transformation given
    /// as a [Mat4].
    ///
    /// Arguments:
    ///
    /// * `glyph`: The character that should be converted to a mesh.
    /// * `flat`: Set this to `true` for 2D meshes, or to `false` in order
    /// to generate a mesh with a depth of `1.0` units.
    /// * `transform`: The 4x4 homogenous transformation matrix.
    ///
    /// Returns:
    ///
    /// The desired [Mesh] or an [MeshTextError] if anything went wrong in the
    /// process.
    pub(crate) fn generate_glyph_with_glam_transform(
        &mut self,
        glyph: char,
        flat: bool,
        transform: &Mat4,
    ) -> Result<Mesh, Box<dyn MeshTextError>> {
        let mut mesh = self.make_mesh(glyph, flat)?;

        for v in mesh.0.iter_mut() {
            *v = transform.transform_point3a(*v);
        }
        mesh.1.transform(transform);

        Ok(mesh)
    }

    /// Generates the [Mesh2D] of a single character with a custom transformation given
    /// as a [Mat3].
    ///
    /// Arguments:
    ///
    /// * `glyph`: The character that should be converted to a mesh.
    /// * `transform`: The 3x3 homogenous transformation matrix.
    ///
    /// Returns:
    ///
    /// The desired [Mesh2D] or an [MeshTextError] if anything went wrong in the
    /// process.
    pub(crate) fn generate_glyph_with_glam_transform_2d(
        &mut self,
        glyph: char,
        transform: &Mat3,
    ) -> Result<Mesh2D, Box<dyn MeshTextError>> {
        let mesh = self.make_mesh(glyph, true)?;
        let mut mesh = mesh_to_flat_2d(mesh);

        for v in mesh.0.iter_mut() {
            *v = transform.transform_point2(*v);
        }
        mesh.1.transform_2d(transform);

        Ok(mesh)
    }

    /// Generates the [IndexedMesh] of a single character with a custom transformation given
    /// as a [Mat4].
    ///
    /// This function handles indexed meshes.
    ///
    /// Arguments:
    ///
    /// * `glyph`: The character that should be converted to a mesh.
    /// * `flat`: Set this to `true` for 2D meshes, or to `false` in order
    /// to generate a mesh with a depth of `1.0` units.
    /// * `transform`: The 4x4 homogenous transformation matrix.
    ///
    /// Returns:
    ///
    /// The desired [IndexedMesh] or an [MeshTextError] if anything went wrong in the
    /// process.
    pub(crate) fn generate_glyph_with_glam_transform_indexed(
        &mut self,
        glyph: char,
        flat: bool,
        transform: &Mat4,
    ) -> Result<IndexedMesh, Box<dyn MeshTextError>> {
        let mut mesh = self.make_indexed_mesh(glyph, flat)?;

        for v in mesh.1.iter_mut() {
            *v = transform.transform_point3a(*v);
        }
        mesh.2.transform(transform);

        Ok(mesh)
    }

    /// Generates the [IndexedMesh2D] of a single character with a custom transformation given
    /// as a [Mat3].
    ///
    /// This function handles indexed meshes.
    ///
    /// Arguments:
    ///
    /// * `glyph`: The character that should be converted to a mesh.
    /// * `transform`: The 3x3 homogenous transformation matrix.
    ///
    /// Returns:
    ///
    /// The desired [IndexedMesh2D] or an [MeshTextError] if anything went wrong in the
    /// process.
    pub(crate) fn generate_glyph_with_glam_transform_indexed_2d(
        &mut self,
        glyph: char,
        transform: &Mat3,
    ) -> Result<IndexedMesh2D, Box<dyn MeshTextError>> {
        let mesh = self.make_indexed_mesh(glyph, true)?;
        let mut mesh = mesh_to_indexed_flat_2d(mesh);

        for v in mesh.1.iter_mut() {
            *v = transform.transform_point2(*v);
        }
        mesh.2.transform_2d(transform);

        Ok(mesh)
    }

    /// Generates a new [Mesh] from the loaded font and the given `glyph`
    /// and inserts it into the internal `cache`.
    ///
    /// Arguments:
    ///
    /// * `glyph`: The character that should be loaded.
    /// * `flat`: Wether the character should be laid out in a 2D mesh.
    ///
    /// Returns:
    ///
    /// A [Result] containing the [Mesh] if successful, otherwise an [MeshTextError].
    fn make_mesh(
        &mut self,
        glyph: char,
        flat: bool,
    ) -> Result<Mesh, Box<dyn MeshTextError>> {
        let font_height = self.font.height() as f32;
        let mut builder = GlyphOutlineBuilder::new(font_height, self.quality);

        let glyph_index = self.glyph_id_of_char(glyph);

        let mut depth = (0.5f32, -0.5f32);
        let (rect, mesh) = match self.font.outline_glyph(glyph_index, &mut builder) {
            Some(bbox) => {
                let mesh = raster_to_mesh(&builder.get_glyph_outline(), flat)?;
                (bbox, mesh)
            }
            None => {
                // The glyph has no outline so it is most likely a space or any other
                // charcter that can not be displayed.
                // An empty mesh is cached for simplicity nevertheless.
                depth = (0f32, 0f32);
                (
                    ttf_parser::Rect {
                        x_min: 0,
                        y_min: 0,
                        x_max: 0,
                        y_max: 0,
                    },
                    Vec::new(),
                )
            }
        };

        // Add mesh to cache.
        let bbox = if flat {
            BoundingBox {
                max: Vec3A::new(
                    rect.x_max as f32 / font_height,
                    rect.y_max as f32 / font_height,
                    0f32,
                ),
                min: Vec3A::new(
                    rect.x_min as f32 / font_height,
                    rect.y_min as f32 / font_height,
                    0f32,
                ),
            }
        } else {
            BoundingBox {
                max: Vec3A::new(
                    rect.x_max as f32 / font_height,
                    rect.y_max as f32 / font_height,
                    depth.0,
                ),
                min: Vec3A::new(
                    rect.x_min as f32 / font_height,
                    rect.y_min as f32 / font_height,
                    depth.1,
                ),
            }
        };

        Ok((mesh, bbox))
    }

    /// Generates a new [IndexedMesh] from the loaded font and the given `glyph`
    /// and inserts it into the internal `cache`.
    ///
    /// Arguments:
    ///
    /// * `glyph`: The character that should be loaded.
    /// * `flat`: Wether the character should be laid out in a 2D mesh.
    ///
    /// Returns:
    ///
    /// A [Result] containing the [IndexedMesh] if successful, otherwise an [MeshTextError].
    fn make_indexed_mesh(
        &mut self,
        glyph: char,
        flat: bool,
    ) -> Result<IndexedMesh, Box<dyn MeshTextError>> {
        let font_height = self.font.height() as f32;
        let mut builder = GlyphOutlineBuilder::new(font_height, self.quality);

        let glyph_index = self.glyph_id_of_char(glyph);

        let mut depth = (0.5f32, -0.5f32);
        let (rect, vertices, indices) = match self.font.outline_glyph(glyph_index, &mut builder) {
            Some(bbox) => {
                let mesh = raster_to_mesh_indexed(&builder.get_glyph_outline(), flat)?;
                (bbox, mesh.0, mesh.1)
            }
            None => {
                // The glyph has no outline so it is most likely a space or any other
                // charcter that can not be displayed.
                // An empty mesh is cached for simplicity nevertheless.
                depth = (0f32, 0f32);
                (
                    ttf_parser::Rect {
                        x_min: 0,
                        y_min: 0,
                        x_max: 0,
                        y_max: 0,
                    },
                    Vec::new(),
                    Vec::new(),
                )
            }
        };

        // Add mesh to cache.
        let bbox = if flat {
            BoundingBox {
                max: Vec3A::new(
                    rect.x_max as f32 / font_height,
                    rect.y_max as f32 / font_height,
                    0f32,
                ),
                min: Vec3A::new(
                    rect.x_min as f32 / font_height,
                    rect.y_min as f32 / font_height,
                    0f32,
                ),
            }
        } else {
            BoundingBox {
                max: Vec3A::new(
                    rect.x_max as f32 / font_height,
                    rect.y_max as f32 / font_height,
                    depth.0,
                ),
                min: Vec3A::new(
                    rect.x_min as f32 / font_height,
                    rect.y_min as f32 / font_height,
                    depth.1,
                ),
            }
        };

        Ok((indices, vertices, bbox))
    }

    /// Finds the [GlyphId] of a certain [char].
    ///
    /// Arguments:
    ///
    /// * `glyph`: The character of which the id is determined.
    ///
    /// Returns:
    ///
    /// The corresponding [GlyphId].
    fn glyph_id_of_char(&self, glyph: char) -> GlyphId {
        self.font
            .glyph_index(glyph)
            .unwrap_or(ttf_parser::GlyphId(0))
    }
}

impl Glyph<MeshText> for MeshGenerator {
    fn generate_glyph(
        &mut self,
        glyph: char,
        flat: bool,
        transform: Option<&[f32; 16]>,
    ) -> Result<MeshText, Box<dyn MeshTextError>> {
        self.generate_glyph(glyph, flat, transform)
    }

    fn generate_glyph_2d(
        &mut self,
        glyph: char,
        transform: Option<&[f32; 9]>,
    ) -> Result<MeshText, Box<dyn MeshTextError>> {
        self.generate_glyph_2d(glyph, transform)
    }
}

impl Glyph<IndexedMeshText> for MeshGenerator {
    fn generate_glyph(
        &mut self,
        glyph: char,
        flat: bool,
        transform: Option<&[f32; 16]>,
    ) -> Result<IndexedMeshText, Box<dyn MeshTextError>> {
        self.generate_glyph_indexed(glyph, flat, transform)
    }

    fn generate_glyph_2d(
        &mut self,
        glyph: char,
        transform: Option<&[f32; 9]>,
    ) -> Result<IndexedMeshText, Box<dyn MeshTextError>> {
        self.generate_glyph_indexed_2d(glyph, transform)
    }
}
