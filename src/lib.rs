#![doc(html_favicon_url = "https://raw.githubusercontent.com/FrankenApps/meshtext/master/logo.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/FrankenApps/meshtext/master/logo.png")]
//! Generate 2d/3d triangle meshes for font glyphs.
//!
//! Supports any outline font supported by ttf_parser.

#![feature(array_chunks)]

/// A bounding box for a mesh. If the mesh is flat, the z-coordinates will be zero.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BoundingBox {
    /// The coordinates of the minimum point.
    pub mins: [f32; 3],
    /// The coordinates of the maximum point.
    pub maxs: [f32; 3],
}

impl BoundingBox {
    /// Creates a new [BoundingBox].
    ///
    /// Arguments:
    /// * `mins`: The minimum vertex of this bounding box.
    /// * `maxs`: The maximum vertex of this bounding box.
    ///
    /// Returns:
    /// The new [BoundingBox].
    pub fn new(mins: [f32; 3], maxs: [f32; 3]) -> Self {
        Self{mins, maxs}
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// An error that can occur while triangulating the outline of a font.
#[derive(Debug)]
pub enum Error {
    Tessellation(lt::TessellationError),
}

impl std::error::Error for Error { }

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Tessellation(e)
                => write!(f, "The glyph outline could not be tesselated: {e}"),
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

pub type FaceRef<'f> = &'f ttf_parser::Face<'f>;
pub use ttf_parser::GlyphId;

/// Generates glyph meshes for a font.
pub struct MeshGenerator<'face> {
    /// The current [Face].
    face: FaceRef<'face>,

    /// Quality settings for generating the text meshes.
    quality: QualitySettings,
}

use lyon_tessellation::{self as lt, path as ltp, path::builder as ltpb};

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
        let scale = 1. / self.face.height() as f32;

        let path_builder = ltpb::NoAttributes::wrap(ltp::path::BuilderImpl::new())
            .flattened(lt::FillOptions::DEFAULT_TOLERANCE)
            .transformed(lt::geom::Scale::new(scale));
        let mut bridge = Bridge(path_builder);
        let Some(bbox) = self.face.outline_glyph(glyph, &mut bridge) else {
            return Ok(Mesh::default());
        };

        let z = if flat {0.0} else {0.5};
        let bbox = BoundingBox::new(
            [bbox.x_min as f32 * scale, bbox.y_min as f32 * scale, -z],
            [bbox.x_max as f32 * scale, bbox.y_max as f32 * scale,  z],
        );

        let mut bufs = lt::VertexBuffers::<[f32; 3], u32>::new();

        let v_base = bufs.vertices.len() as u32;
        let i_base = bufs.vertices.len() as u32;

        let path = bridge.0.build();
        let mut tess = lt::FillTessellator::new();
        let opts = lt::FillOptions::default()
            .with_fill_rule(lt::FillRule::NonZero);

        let mut buf_builder = lt::BuffersBuilder::new(
            &mut bufs,
            |v: lt::FillVertex<'_>| -> [f32; 3] {
                let [x, y]: [f32; 2] = v.position().into();
                [x, y, z]
            }
        );
        tess.tessellate_path(&path, &opts, &mut buf_builder)
            .map_err(|e| Error::Tessellation(e))?;

        if !flat {
            // find boundary edges
            let mut edge_set = std::collections::HashMap::new();
            bufs.indices[i_base as usize ..]
                .array_chunks().copied()
                .flat_map(|[a, b, c]| [(a, b), (b, c), (c, a)])
                .for_each(|(a, b)| {
                    let key = if b < a {(b, a)} else {(a, b)};
                    use std::collections::hash_map::Entry;
                    match edge_set.entry(key) {
                        Entry::Occupied(e) => { e.remove(); },
                        Entry::Vacant(e)   => { e.insert(()); },
                    }
                });

            // add rear face
            let v_rear_base = bufs.vertices.len();
            bufs.vertices.extend_from_within(v_base as usize ..);
            for v in &mut bufs.vertices[v_rear_base..] { v[2] = -z; }

            let i_rear_base = bufs.indices.len();
            bufs.indices.extend_from_within(i_base as usize ..);
            for [a, _, c] in bufs.indices[i_rear_base..].array_chunks_mut() {
                std::mem::swap(a, c);
            }

            // add sides
            let r = v_rear_base as u32 - v_base;
            bufs.indices.extend(
                edge_set.into_keys()
                    .flat_map(|(a, b)| [a, b, b+r, a+r, a, b+r])
            );
        }

        let lt::VertexBuffers{indices, vertices} = bufs;
        Ok(Mesh{bbox, indices, vertices})
    }
}

struct Bridge<B>(ltpb::NoAttributes<B>) where
    B: ltpb::PathBuilder;

impl<B> ttf_parser::OutlineBuilder for Bridge<B> where
    B: ltpb::PathBuilder,
{
    fn move_to(&mut self, x: f32, y: f32) { self.0.begin([x, y].into()); }
    fn line_to(&mut self, x: f32, y: f32) { self.0.line_to([x, y].into()); }
    fn close(&mut self) { self.0.close(); }

    fn quad_to(&mut self, xc: f32, yc: f32, x: f32, y: f32) {
        self.0.quadratic_bezier_to([xc, yc].into(), [x, y].into());
    }

    fn curve_to(&mut self, xc0: f32, yc0: f32, xc1: f32, yc1: f32, x: f32, y: f32) {
        self.0.cubic_bezier_to([xc0, yc0].into(), [xc1, yc1].into(), [x, y].into());
    }
}

