#![doc(html_favicon_url = "https://raw.githubusercontent.com/FrankenApps/meshtext/master/logo.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/FrankenApps/meshtext/master/logo.png")]
//! Generate 2D or 3D triangle meshes from text.
//!
//! Generate vertices or indices and vertices for a
//! [vertex-vertex mesh](https://en.wikipedia.org/wiki/Polygon_mesh#Vertex-vertex_meshes).
//!
//! - Supports [TrueType](https://docs.microsoft.com/en-us/typography/truetype/),
//! [OpenType](https://docs.microsoft.com/en-us/typography/opentype/spec/)
//! and [AAT](https://developer.apple.com/fonts/TrueType-Reference-Manual/RM06/Chap6AATIntro.html)
//! fonts
//! - Handles caching of characters that were already triangulated
//! - Allows transforming text sections
//! - Fully customizable to easily integrate in your rendering pipeline

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

pub type Result<T> = std::result::Result<T, Error>;

mod mesh_generator;
pub use mesh_generator::MeshGenerator;

mod bounding_box;
pub use bounding_box::BoundingBox;

type Point = (f32, f32);

/// The internal representation of a rasterized glyph outline.
pub(crate) struct GlyphOutline {
    /// The indices that form closed contours of points.
    pub contours: Vec<Vec<u32>>,

    /// A point cloud that contains one or more contours.
    pub points: Vec<Point>,
}

/// Holds the generated mesh data for the given glyph.
///
/// The triangles use indexed vertices.
pub struct GlyphMesh {
    /// The bounding box of this mesh.
    pub bbox: BoundingBox,

    /// The indices of this mesh.
    pub indices: Vec<u32>,

    /// The vertices of this mesh.
    pub vertices: Vec<f32>,
}

/// Controls the quality of generated glyphs.
///
/// Generally each setting can be tweaked to generate better
/// looking glyphs at the cost of a certain performance impact.
#[derive(Debug, Clone, Copy)]
pub struct QualitySettings {
    /// The number of linear interpolation steps performed
    /// on a _quadratic bezier curve_.
    ///
    /// If the specified font does not use _quadratic splines_
    /// this setting will have no effect.
    ///
    /// Higher values result in higher polygon count.
    pub quad_interpolation_steps: u32,

    /// The number of quadratic interpolation steps performed
    /// on a _cubic bezier curve_.
    ///
    /// If the specified font does not use _cubic splines_
    /// this setting will have no effect.
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

