mod outline_builder;

use {
    crate::*,
    glam::Vec3A,
    ttf_parser::Face,
    outline_builder::GlyphOutlineBuilder,
};

/// A [MeshGenerator] handles rasterizing individual glyphs.
///
/// Each [MeshGenerator] will handle exactly one font. This means
/// if you need support for multiple fonts, you will need to create
/// multiple instances (one per font) of this generator.
pub struct MeshGenerator<'face> {
    /// The current [Face].
    face: &'face Face<'face>,

    /// Quality settings for generating the text meshes.
    quality: QualitySettings,
}

impl<'face> MeshGenerator<'face> {
    /// Creates a new [MeshGenerator].
    ///
    /// Arguments:
    ///
    /// * `font`: The font that will be used for rasterizing.
    pub fn new(face: &'face Face<'face>) -> Self {
        Self{face, quality: QualitySettings::default()}
    }

    /// Creates a new [MeshGenerator] with custom quality settings.
    ///
    /// Arguments:
    ///
    /// * `font`: The font that will be used for rasterizing.
    /// * `quality`: The [QualitySettings] that should be used.
    pub fn new_with_quality(face: &'face Face<'face>, quality: QualitySettings) -> Self {
        Self{face, quality}
    }

    /// Get the face used by this [MeshGenerator].
    pub fn face(&self) -> &'face Face<'face> {
        self.face
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
    pub fn generate_indexed_mesh(
        &mut self,
        glyph: char,
        flat: bool,
    ) -> Result<GlyphMesh> {
        let font_height = self.face.height() as f32;
        let mut builder = GlyphOutlineBuilder::new(font_height, self.quality);

        let glyph_index = self.face
            .glyph_index(glyph)
            .unwrap_or_default();

        let mut depth = (0.5f32, -0.5f32);
        let (rect, vertices, indices) = match self.face.outline_glyph(glyph_index, &mut builder) {
            Some(bbox) => {
                let mesh = tesselate(&builder.get_glyph_outline(), flat)?;
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

        // Compute bounding box.
        if flat { depth = (0., 0.); }
        let bbox = BoundingBox {
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
        };

        let mesh_text = GlyphMesh {
            bbox,
            indices,
            vertices: vertices.into_iter()
                .map(Into::<[f32; 3]>::into)
                .flatten()
                .collect(),
        };
        Ok(mesh_text)
    }
}

/// Generates an indexed triangle mesh from a discrete [GlyphOutline].
///
/// Arguments:
///
/// * `outline`: The outline of the desired glyph.
/// * `flat`: Generates a two dimensional mesh if `true`, otherwise
/// a three dimensional mesh with depth `1.0` units is generated.
///
/// Returns:
///
/// A [Result] containing the generated mesh data or an [MeshTextError] if
/// anything went wrong in the process.
#[allow(unused)]
fn tesselate(
    outline: &GlyphOutline,
    flat: bool,
) -> Result<(Vec<Vec3A>, Vec<u32>)> {
    let points = &outline.points;
    let (triangles, edges) = get_glyph_area_triangulation(outline)?;

    if flat {
        let mut vertices = Vec::new();
        for p in points {
            vertices.push(Vec3A::new(p.0 as f32, p.1 as f32, 0f32));
        }

        let mut indices = Vec::new();
        for i in triangles {
            indices.push(i.0 as u32);
            indices.push(i.1 as u32);
            indices.push(i.2 as u32);
        }

        Ok((vertices, indices))
    } else {
        let mut vertices = Vec::new();
        for p in points {
            vertices.push(Vec3A::new(p.0 as f32, p.1 as f32, 0.5f32));
        }
        let flat_count = vertices.len() as u32;

        for p in points {
            vertices.push(Vec3A::new(p.0 as f32, p.1 as f32, -0.5f32));
        }

        let mut indices = Vec::new();
        for i in triangles {
            indices.push(i.0 as u32);
            indices.push(i.1 as u32);
            indices.push(i.2 as u32);

            indices.push(i.2 as u32 + flat_count);
            indices.push(i.1 as u32 + flat_count);
            indices.push(i.0 as u32 + flat_count);
        }

        // Add the vertices and indices in between the contours (e.g. in the z-axis).
        let flat_count = (vertices.len() / 2) as u32;

        for e in edges.iter() {
            // First triangle.
            indices.push(e.0 as u32);
            indices.push(e.1 as u32);
            indices.push(flat_count + e.1 as u32);

            // Second triangle.
            indices.push(flat_count + e.0 as u32);
            indices.push(e.0 as u32);
            indices.push(flat_count + e.1 as u32);
        }

        Ok((vertices, indices))
    }
}

fn get_glyph_area_triangulation(
    outline: &GlyphOutline,
) -> Result<(Vec<(usize, usize, usize)>, Vec<(usize, usize)>)> {
    // TODO: Implement a custom triangulation algorithm to get rid of these conversions.
    let points: Vec<(f64, f64)> = outline
        .points
        .iter()
        .map(|p| (p.0 as f64, p.1 as f64))
        .collect();
    let mut contours = Vec::new();
    for c in outline.contours.iter() {
        let path_indices: Vec<usize> = c.iter().map(|i| *i as usize).collect();
        contours.push(path_indices);
    }

    // We might need access to the edges later, so we compute them here once.
    let mut edges = Vec::new();
    for c in contours.iter() {
        let next = edges.len();
        for (a, b) in c.iter().zip(c.iter().skip(1)) {
            edges.push((*a, *b));
        }
        if let Some(start) = edges.get(next) {
            if start.0 != edges.last().unwrap().1 {
                return Err(crate::Error::Triangulation(cdt::Error::OpenContour));
            }
        }
    }

    // Triangulate the contours.
    let triangles = match cdt::triangulate_with_edges(&points, &edges) {
        Ok(result) => result,
        Err(err) => return Err(Error::Triangulation(err)),
    };
    Ok((triangles, edges))
}

