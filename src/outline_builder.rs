use crate::QualitySettings;

type Point = (f32, f32);

/// The internal representation of a rasterized glyph outline.
pub(crate) struct Outline {
    /// The indices that form closed contours of points.
    pub contours: Vec<Vec<usize>>,

    /// A point cloud that contains one or more contours.
    pub points: Vec<Point>,
}

pub(crate) struct OutlineBuilder {
    font_height: f32,
    quality: QualitySettings,

    contours: Vec<Vec<usize>>,
    current_contour: Vec<usize>,
    points: Vec<Point>,
    current_point: Point,
}

impl OutlineBuilder {
    pub(crate) fn new(font_height: f32, quality: QualitySettings) -> Self {
        Self {
            font_height,
            quality,

            contours: Vec::new(),
            current_contour: Vec::new(),
            points: Vec::new(),
            current_point: (0f32, 0f32),
        }
    }

    pub(crate) fn into_outline(self) -> Outline {
        Outline{contours: self.contours, points: self.points}
    }

    fn add_segment(&mut self, (x, y): (f32, f32)) {
        self.current_point = (x, y);
        self.current_contour.push(self.points.len());
        self.points.push((x / self.font_height, y / self.font_height));
    }
}

impl ttf_parser::OutlineBuilder for OutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        // TODO detect unclosed contour here
        self.add_segment((x, y));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.add_segment((x, y));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) {
        // first point already generated
        for step in 1..=self.quality.quad_interpolation_steps {
            let t = step as f32 / self.quality.quad_interpolation_steps as f32;
            let (x, y) = point_on_quad(self.current_point, (x1, y1), (x2, y2), t);
            self.line_to(x, y);
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) {
        // first point already generated
        for step in 1..=self.quality.cubic_interpolation_steps {
            let t = step as f32 / self.quality.cubic_interpolation_steps as f32;
            let (x, y) = point_on_cubic(self.current_point, (x1, y1), (x2, y2), (x3, y3), t);
            self.line_to(x, y);
        }
    }

    fn close(&mut self) {
        // remove redundant end point
        self.points.pop();
        self.current_contour.pop();

        self.current_contour.push(self.current_contour[0]);
        self.contours.push(self.current_contour.split_off(0));
    }
}

fn lerp_points((ax, ay): Point, (bx, by): Point, t: f32) -> Point {
    (ax - (ax - bx) * t, ay - (ay - by) * t)
}

fn point_on_quad(p0: Point, p1: Point, p2: Point, t: f32) -> Point {
    let a = lerp_points(p0, p1, t);
    let b = lerp_points(p1, p2, t);
    lerp_points(a, b, t)
}

fn point_on_cubic(p0: Point, p1: Point, p2: Point, p3: Point, t: f32) -> Point {
    let a = point_on_quad(p0, p1, p2, t);
    let b = point_on_quad(p1, p2, p3, t);
    lerp_points(a, b, t)
}

