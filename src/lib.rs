#![doc(html_favicon_url = "https://raw.githubusercontent.com/FrankenApps/meshtext/master/logo.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/FrankenApps/meshtext/master/logo.png")]
//! Generate 2d/3d triangle meshes for font glyphs.
//!
//! Supports any outline font supported by ttf_parser.

mod outline_builder;
use outline_builder::{Outline, OutlineBuilder};

mod bounding_box;
pub use bounding_box::BoundingBox;

pub type Result<T> = std::result::Result<T, Error>;

/// An error that can occur while triangulating the outline of a font.
#[derive(Debug)]
pub enum Error {
    Triangulation(cdt::Error),
}

impl std::error::Error for Error { }

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Triangulation(cdte)
                => write!(f, "The glyph outline could not be triangulated: {}", cdte),
        }
    }
}

/// Holds the generated mesh data for the given glyph.
///
/// The triangles use indexed vertices.
#[derive(Default)]
pub struct Mesh {
    /// The bounding box of this mesh.
    pub bbox: BoundingBox,

    /// The indices of this mesh.
    pub indices: Vec<u32>,

    /// The vertices of this mesh.
    pub vertices: Vec<[f32; 3]>,
}

/// Controls the quality of generated glyphs.
///
/// Generally each setting can be tweaked to generate better looking glyphs at the cost of a
/// certain performance impact.
#[derive(Debug, Clone, Copy)]
pub struct QualitySettings {
    /// The number of linear interpolation steps performed on any _quadratic bezier curves_ present
    /// in the font.
    ///
    /// Higher values result in higher polygon count.
    pub quad_interpolation_steps: u32,

    /// The number of quadratic interpolation steps performed on any _cubic bezier curves_ present
    /// in the font.
    ///
    /// Higher values result in higher polygon count.
    pub cubic_interpolation_steps: u32,
}

impl Default for QualitySettings {
    fn default() -> Self {
        Self {
            quad_interpolation_steps: 5,
            cubic_interpolation_steps: 3,
        }
    }
}

use glam::Vec3A;

pub type FaceRef<'f> = &'f ttf_parser::Face<'f>;
pub use ttf_parser::GlyphId;

/// Generates glyph meshes for a font.
pub struct MeshGenerator<'face> {
    /// The current [Face].
    face: FaceRef<'face>,

    /// Quality settings for generating the text meshes.
    quality: QualitySettings,
}

impl<'face> MeshGenerator<'face> {
    /// Creates a new [MeshGenerator].
    ///
    /// Arguments:
    /// * `font`: The font that will be used for rasterizing.
    pub fn new(face: FaceRef<'face>) -> Self {
        Self{face, quality: QualitySettings::default()}
    }

    /// Creates a new [MeshGenerator] with custom quality settings.
    ///
    /// Arguments:
    /// * `font`: The font that will be used for rasterizing.
    /// * `quality`: The [QualitySettings] that should be used.
    pub fn new_with_quality(face: FaceRef<'face>, quality: QualitySettings) -> Self {
        Self{face, quality}
    }

    /// Get the face used by this [MeshGenerator].
    pub fn face(&self) -> FaceRef<'face> {
        self.face
    }

    /// Generates a new [Mesh] from the loaded font and the given `glyph`.
    ///
    /// Arguments:
    /// * `glyph`: The glyph to be meshed.
    /// * `flat`: Wether the character should be laid out in a 2D mesh.
    ///
    /// Returns:
    /// A [Result] containing the [Mesh] if successful, otherwise an [Error].
    pub fn generate_mesh(&self, glyph: GlyphId, flat: bool) -> Result<Mesh> {
        let font_height = self.face.height() as f32;
        let mut builder = OutlineBuilder::new(font_height, self.quality);

        let Some(bbox) = self.face.outline_glyph(glyph, &mut builder) else {
            return Ok(Mesh::default());
        };

        let (vertices, indices) = tesselate(builder.into_outline(), flat)?;

        // Compute bounding box.
        let depth = if flat {(0., 0.)} else {(0.5, -0.5)};
        let bbox = BoundingBox {
            max: Vec3A::new(
                bbox.x_max as f32 / font_height,
                bbox.y_max as f32 / font_height,
                depth.0,
            ),
            min: Vec3A::new(
                bbox.x_min as f32 / font_height,
                bbox.y_min as f32 / font_height,
                depth.1,
            ),
        };

        let vertices = vertices.into_iter()
            .map(Into::<[f32; 3]>::into)
            .collect();
        Ok(Mesh {bbox, indices, vertices})
    }
}

/// Generates an indexed triangle mesh from a discrete [Outline].
///
/// Arguments:
/// * `outline`: The outline of the desired glyph.
/// * `flat`: Generates a two dimensional mesh if `true`, otherwise a three dimensional mesh
/// with depth `1.0` units is generated.
///
/// Returns:
/// A [Result] containing the generated mesh data or an [Error] upon failure.
fn tesselate(outline: Outline, flat: bool) -> Result<(Vec<Vec3A>, Vec<u32>)> {
    let triangles = {
        // TODO: Implement a custom triangulation algorithm to get rid of these conversions.
        let points = outline.points.iter().copied()
            .map(|(x, y)| (x as f64, y as f64))
            .collect::<Vec<_>>();

        // Triangulate the contours.
        cdt::triangulate_contours(&points[..], &outline.contours[..])
            .map_err(|e| Error::Triangulation(e))?
    };

    let z = if flat {0.0} else {0.5};

    // front face
    let mut vertices = outline.points.iter().copied()
        .map(|(x, y)| Vec3A::new(x, y, z))
        .collect();

    let mut indices = triangles.iter().copied()
        .flat_map(|(a, b, c)| [a, b, c].map(|i| i as u32))
        .collect();

    if flat {return Ok((vertices, indices));}

    // back face
    let back = vertices.len();
    vertices.extend(
        outline.points.iter().copied()
            .map(|(x, y)| Vec3A::new(x, y, -z))
    );

    indices.extend(
        triangles.iter().copied()
            .flat_map(|(a, b, c)| [c, b, a].map(|i| (i + back) as u32))
    );

    // sides
    indices.extend(
        outline.contours.iter()
            .flat_map(|c| std::iter::zip(&c[0..], &c[1..]))
            .flat_map(|(&i, &j)| [i, j, back + j, back + i, i, back + j])
            .map(|i| i as u32)
    );

    Ok((vertices, indices))
}

