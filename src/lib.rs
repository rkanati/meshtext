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

/// Contains the various errors that may occur
/// while using this crate.
pub mod error {
    use std::{error::Error, fmt};

    /// Any error that can occur while generating a [crate::MeshText] or an [crate::IndexedMeshText].
    pub trait MeshTextError: fmt::Debug + fmt::Display {}

    /// An error that can occur while parsing the outline of a font.
    #[derive(Debug)]
    pub struct GlyphOutlineError;

    impl MeshTextError for GlyphOutlineError {}

    impl Error for GlyphOutlineError {}

    impl fmt::Display for GlyphOutlineError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(
                f,
                "The glyph outline of this font seems to be malformed / unsupported."
            )
        }
    }

    /// An error that can occur while triangulating the outline of a font.
    #[derive(Debug)]
    pub struct GlyphTriangulationError(pub cdt::Error);

    impl MeshTextError for GlyphTriangulationError {}

    impl Error for GlyphTriangulationError {}

    impl fmt::Display for GlyphTriangulationError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "The glyph outline could not be triangulated.")
        }
    }
}

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

/// Holds the generated mesh data for the given text input.
///
/// The triangles use indexed vertices.
pub struct IndexedMeshText {
    /// The bounding box of this mesh.
    pub bbox: BoundingBox,

    /// The indices of this mesh.
    pub indices: Vec<u32>,

    /// The vertices of this mesh.
    pub vertices: Vec<f32>,
}

/// Holds the generated mesh data for the given text input.
pub struct MeshText {
    /// The bounding box of this mesh.
    pub bbox: BoundingBox,

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

mod traits {
    mod glyph;
    pub use glyph::*;
}
pub use traits::*;

pub(crate) mod util {
    mod glam_conversions;
    pub(crate) use glam_conversions::*;

    mod mesh_to_flat_2d;
    pub(crate) use mesh_to_flat_2d::*;

    mod outline_builder;
    pub(crate) use outline_builder::GlyphOutlineBuilder;

    mod raster_to_mesh;
    pub(crate) use raster_to_mesh::*;

    mod text_mesh;
    pub(crate) use text_mesh::*;

    mod triangulation;
    pub(crate) use triangulation::*;
}

