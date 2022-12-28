use glam::Vec3A;

/// A bounding box for a mesh. If the mesh is flat, the z-coordinates will be zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    /// The coordinates of the minimum point.
    pub min: Vec3A,
    /// The coordinates of the maximum point.
    pub max: Vec3A,
}

impl BoundingBox {
    /// Creates a new [BoundingBox].
    ///
    /// Arguments:
    ///
    /// * `min`: The minimum vertex of this bounding box.
    /// * `max`: The maximum vertex of this bounding box.
    ///
    /// Returns:
    ///
    /// The new [BoundingBox].
    pub fn new(min: Vec3A, max: Vec3A) -> Self {
        Self{min, max}
    }

    /// Creates a new empty [BoundingBox].
    ///
    /// Returns:
    ///
    /// The empty [BoundingBox].
    pub(crate) fn empty() -> Self {
        Self {
            max: Vec3A::ZERO,
            min: Vec3A::ZERO,
        }
    }

    /// Calculates the center of this [BoundingBox].
    ///
    /// Returns:
    ///
    /// A [Vec3A] representing the point in the geometric
    /// center of this [BoundingBox].
    ///
    /// # Example
    ///
    /// ```rust
    /// use glam::Vec3A;
    /// use meshtext::BoundingBox;
    ///
    /// let bbox = BoundingBox::new(
    ///     Vec3A::new(0f32, 0f32, 0f32),
    ///     Vec3A::new(1f32, 1f32, 1f32),
    /// );
    ///
    /// assert_eq!(bbox.center(), Vec3A::new(0.5, 0.5, 0.5));
    /// ```
    pub fn center(&self) -> Vec3A {
        self.min + (self.max - self.min) * 0.5f32
    }

    /// Gets the size of this [BoundingBox].
    ///
    /// Returns:
    ///
    /// A [Vec3A] with the extent of this [BoundingBox]
    /// along each coordinate axis.
    ///
    /// # Example
    ///
    /// ```rust
    /// use glam::Vec3A;
    /// use meshtext::BoundingBox;
    ///
    /// let bbox = BoundingBox::new(
    ///     Vec3A::new(0f32, 0f32, 1f32),
    ///     Vec3A::new(1f32, 1f32, 3f32),
    /// );
    ///
    /// assert_eq!(bbox.size(), Vec3A::new(1f32, 1f32, 2f32));
    /// ```
    pub fn size(&self) -> Vec3A {
        (self.max - self.min).abs()
    }
}

