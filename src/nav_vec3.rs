use crate::{nav_mesh::NavConnection, Scalar, ZERO_TRESHOLD};
use approx::{AbsDiffEq, RelativeEq, UlpsEq};
use serde::{Deserialize, Serialize};
use spade::PointN;
use std::ops::{Add, Div, Mul, Neg, Sub};

#[cfg(feature = "parallel")]
macro_rules! into_iter {
    ($v:expr) => {
        $v.into_par_iter()
    };
}
#[cfg(not(feature = "parallel"))]
macro_rules! into_iter {
    ($v:expr) => {
        $v.into_iter()
    };
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavVec3 {
    pub x: Scalar,
    pub y: Scalar,
    pub z: Scalar,
}

impl NavVec3 {
    #[inline]
    pub fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self { x, y, z }
    }

    #[inline]
    pub fn sqr_magnitude(self) -> Scalar {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    #[inline]
    pub fn magnitude(self) -> Scalar {
        self.sqr_magnitude().sqrt()
    }

    #[inline]
    pub fn same_as(self, other: Self) -> bool {
        (other - self).sqr_magnitude() < ZERO_TRESHOLD
    }

    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self {
            x: (self.y * other.z) - (self.z * other.y),
            y: (self.z * other.x) - (self.x * other.z),
            z: (self.x * other.y) - (self.y * other.x),
        }
    }

    #[inline]
    pub fn dot(self, other: Self) -> Scalar {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    #[inline]
    pub fn normalize(self) -> Self {
        let len = self.magnitude();
        if len < ZERO_TRESHOLD {
            Self::new(0.0, 0.0, 0.0)
        } else {
            Self::new(self.x / len, self.y / len, self.z / len)
        }
    }

    #[inline]
    pub fn abs(self) -> Self {
        Self::new(self.x.abs(), self.y.abs(), self.z.abs())
    }

    #[inline]
    pub fn project(self, from: Self, to: Self) -> Scalar {
        let diff = to - from;
        (self - from).dot(diff) / diff.sqr_magnitude()
    }

    #[inline]
    pub fn unproject(from: Self, to: Self, t: Scalar) -> Self {
        let diff = to - from;
        from + Self::new(diff.x * t, diff.y * t, diff.z * t)
    }

    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }

    #[inline]
    pub fn distance_to_plane(self, origin: Self, normal: Self) -> Scalar {
        normal.dot(self - origin)
    }

    #[inline]
    pub fn is_above_plane(self, origin: Self, normal: Self) -> bool {
        self.distance_to_plane(origin, normal) > -ZERO_TRESHOLD
    }

    pub fn project_on_plane(self, origin: Self, normal: Self) -> Self {
        let v = self - origin;
        let n = normal.normalize();
        let dot = v.dot(n);
        let d = NavVec3::new(normal.x * dot, normal.y * dot, normal.z * dot);
        self - d
    }

    pub fn raycast_plane(from: Self, to: Self, origin: Self, normal: Self) -> Option<Self> {
        let dir = (to - from).normalize();
        let denom = normal.dot(dir);
        if denom.abs() > ZERO_TRESHOLD {
            let t = (origin - from).dot(normal) / denom;
            if t >= 0.0 && t <= (to - from).magnitude() {
                return Some(from + dir * t);
            }
        }
        None
    }

    pub fn raycast_line(from: Self, to: Self, a: Self, b: Self, normal: Self) -> Option<Self> {
        let p = Self::raycast_plane(from, to, a, normal)?;
        let t = p.project(a, b).max(0.0).min(1.0);
        Some(Self::unproject(a, b, t))
    }

    pub fn raycast_line_exact(
        from: Self,
        to: Self,
        a: Self,
        b: Self,
        normal: Self,
    ) -> Option<Self> {
        let p = Self::raycast_plane(from, to, a, normal)?;
        let t = p.project(a, b);
        if t >= 0.0 && t <= 1.0 {
            Some(Self::unproject(a, b, t))
        } else {
            None
        }
    }

    pub fn raycast_triangle(from: Self, to: Self, a: Self, b: Self, c: Self) -> Option<Self> {
        let tab = (b - a).normalize();
        let tbc = (c - b).normalize();
        let tca = (a - c).normalize();
        let n = tab.cross(tbc).normalize();
        let contact = Self::raycast_plane(from, to, a, n)?;
        let nab = tab.cross(n);
        let nbc = tbc.cross(n);
        let nca = tca.cross(n);
        if contact.is_above_plane(a, -nab)
            && contact.is_above_plane(b, -nbc)
            && contact.is_above_plane(c, -nca)
        {
            Some(contact)
        } else {
            None
        }
    }

    /// line: (origin, normal)
    pub fn planes_intersection(p1: Self, n1: Self, p2: Self, n2: Self) -> Option<(Self, Self)> {
        let u = n1.cross(n2);
        if u.sqr_magnitude() < ZERO_TRESHOLD {
            return None;
        }
        let a = u.abs();
        let mc = if a.x > a.y {
            if a.x > a.z {
                1
            } else {
                3
            }
        } else if a.y > a.z {
            2
        } else {
            3
        };
        let d1 = -n1.dot(p1);
        let d2 = -n2.dot(p2);
        let p = match mc {
            1 => Some(Self::new(
                0.0,
                (d2 * n1.z - d1 * n2.z) / u.x,
                (d1 * n2.y - d2 * n1.y) / u.x,
            )),
            2 => Some(Self::new(
                (d1 * n2.z - d2 * n1.z) / u.y,
                0.0,
                (d2 * n1.x - d1 * n2.x) / u.y,
            )),
            3 => Some(Self::new(
                (d2 * n1.y - d1 * n2.y) / u.z,
                (d1 * n2.x - d2 * n1.x) / u.z,
                0.0,
            )),
            _ => None,
        }?;
        Some((p, u.normalize()))
    }

    /// (from: (point, on edge connection), to: (point, on edge connection), normal)
    pub fn triangles_intersection(
        a1: Self,
        b1: Self,
        c1: Self,
        a2: Self,
        b2: Self,
        c2: Self,
    ) -> Option<(
        (Self, Option<NavConnection>),
        (Self, Option<NavConnection>),
        Self,
    )> {
        let tab1 = (b1 - a1).normalize();
        let tbc1 = (c1 - b1).normalize();
        let n1 = tab1.cross(tbc1).normalize();
        let tab2 = (b2 - a2).normalize();
        let tbc2 = (c2 - b2).normalize();
        let n2 = tab2.cross(tbc2).normalize();
        let contacts = into_iter!([(a2, b2), (b2, c2), (c2, a2)])
            .filter_map(|(from, to)| Self::raycast_plane(*from, *to, a1, n1))
            .collect::<Vec<_>>();
        let mut deduplicated = Vec::with_capacity(contacts.len());
        'root: for i in 0..contacts.len() {
            for j in (i + 1)..contacts.len() {
                if (contacts[i] - contacts[j]).sqr_magnitude() < ZERO_TRESHOLD {
                    continue 'root;
                }
            }
            deduplicated.push(contacts[i]);
        }
        if deduplicated.len() != 2 {
            return None;
        }
        let sb = deduplicated[0];
        let se = deduplicated[1];
        if !Self::does_line_crosses_triangle(sb, se, a1, b1, c1) {
            return None;
        }
        let no = (n2 * 100.0).project_on_plane(a1, n1).normalize();
        let clipped = into_iter!([(a1, b1, 0, 1), (b1, c1, 1, 2), (c1, a1, 2, 0)])
            .filter_map(|(from, to, index_from, index_to)| {
                let p = Self::raycast_line_exact(*from, *to, sb, se, no)?;
                Some((p, NavConnection(*index_from, *index_to)))
            })
            .collect::<Vec<_>>();
        let (b, e, n) = match clipped.len() {
            2 => {
                if (clipped[1].0 - clipped[0].0).sqr_magnitude() < ZERO_TRESHOLD {
                    None
                } else {
                    Some((
                        (clipped[0].0, Some(clipped[0].1)),
                        (clipped[1].0, Some(clipped[1].1)),
                        no,
                    ))
                }
            }
            1 => {
                let (p, conn) = clipped[0];
                let points = &[a1, b1, c1];
                let pb = points[conn.0 as usize];
                let pe = points[conn.1 as usize];
                let n = (pe - pb).cross(n1).normalize();
                let db = sb.distance_to_plane(pb, n);
                let de = se.distance_to_plane(pb, n);
                if db > de {
                    if (p - se).sqr_magnitude() < ZERO_TRESHOLD {
                        None
                    } else {
                        Some(((p, Some(conn)), (se, None), no))
                    }
                } else {
                    if (p - sb).sqr_magnitude() < ZERO_TRESHOLD {
                        None
                    } else {
                        Some(((sb, None), (p, Some(conn)), no))
                    }
                }
            }
            _ => Some(((sb, None), (se, None), no)),
        }?;
        if n.cross(e.0 - b.0).z >= 0.0 {
            Some((b, e, n))
        } else {
            Some((e, b, n))
        }
    }

    pub fn is_line_between_points(from: Self, to: Self, a: Self, b: Self, normal: Self) -> bool {
        let n = (to - from).cross(normal);
        let sa = Self::side(n.dot(a - from));
        let sb = Self::side(n.dot(b - from));
        sa != sb
    }

    fn does_line_crosses_triangle(from: Self, to: Self, a: Self, b: Self, c: Self) -> bool {
        let tab = (b - a).normalize();
        let tbc = (c - b).normalize();
        let tca = (a - c).normalize();
        let n = tab.cross(tbc).normalize();
        let nab = tab.cross(n);
        let nbc = tbc.cross(n);
        let nca = tca.cross(n);
        (from.is_above_plane(a, -nab)
            && from.is_above_plane(b, -nbc)
            && from.is_above_plane(c, -nca))
            || (to.is_above_plane(a, -nab)
                && to.is_above_plane(b, -nbc)
                && to.is_above_plane(c, -nca))
    }

    fn side(v: Scalar) -> i8 {
        if v.abs() < ZERO_TRESHOLD {
            0
        } else {
            v.signum() as i8
        }
    }
}

impl From<(Scalar, Scalar, Scalar)> for NavVec3 {
    fn from(value: (Scalar, Scalar, Scalar)) -> Self {
        Self {
            x: value.0,
            y: value.1,
            z: value.2,
        }
    }
}

impl From<(Scalar, Scalar)> for NavVec3 {
    fn from(value: (Scalar, Scalar)) -> Self {
        Self {
            x: value.0,
            y: value.1,
            z: 0.0,
        }
    }
}

impl From<[Scalar; 3]> for NavVec3 {
    fn from(value: [Scalar; 3]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
        }
    }
}

impl From<[Scalar; 2]> for NavVec3 {
    fn from(value: [Scalar; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: 0.0,
        }
    }
}

impl Add for NavVec3 {
    type Output = Self;

    #[inline]
    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl Add<Scalar> for NavVec3 {
    type Output = Self;

    #[inline]
    fn add(self, other: Scalar) -> Self {
        Self {
            x: self.x + other,
            y: self.y + other,
            z: self.z + other,
        }
    }
}

impl Sub for NavVec3 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

impl Sub<Scalar> for NavVec3 {
    type Output = Self;

    #[inline]
    fn sub(self, other: Scalar) -> Self {
        Self {
            x: self.x - other,
            y: self.y - other,
            z: self.z - other,
        }
    }
}

impl Mul for NavVec3 {
    type Output = Self;

    #[inline]
    fn mul(self, other: Self) -> Self {
        Self {
            x: self.x * other.x,
            y: self.y * other.y,
            z: self.z * other.z,
        }
    }
}

impl Mul<Scalar> for NavVec3 {
    type Output = Self;

    #[inline]
    fn mul(self, other: Scalar) -> Self {
        Self {
            x: self.x * other,
            y: self.y * other,
            z: self.z * other,
        }
    }
}

impl Div for NavVec3 {
    type Output = Self;

    #[inline]
    fn div(self, other: Self) -> Self {
        Self {
            x: self.x / other.x,
            y: self.y / other.y,
            z: self.z / other.z,
        }
    }
}

impl Div<Scalar> for NavVec3 {
    type Output = Self;

    #[inline]
    fn div(self, other: Scalar) -> Self {
        Self {
            x: self.x / other,
            y: self.y / other,
            z: self.z / other,
        }
    }
}

impl Neg for NavVec3 {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

impl PointN for NavVec3 {
    type Scalar = Scalar;

    fn dimensions() -> usize {
        3
    }

    fn nth(&self, index: usize) -> &Self::Scalar {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => unreachable!(),
        }
    }
    fn nth_mut(&mut self, index: usize) -> &mut Self::Scalar {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => unreachable!(),
        }
    }

    fn from_value(value: Self::Scalar) -> Self {
        NavVec3::new(value, value, value)
    }
}

impl AbsDiffEq for NavVec3 {
    type Epsilon = <Scalar as AbsDiffEq>::Epsilon;

    fn default_epsilon() -> Self::Epsilon {
        Scalar::default_epsilon()
    }

    fn abs_diff_eq(&self, other: &Self, epsilon: Self::Epsilon) -> bool {
        Scalar::abs_diff_eq(&self.x, &other.x, epsilon)
            && Scalar::abs_diff_eq(&self.y, &other.y, epsilon)
            && Scalar::abs_diff_eq(&self.z, &other.z, epsilon)
    }
}

impl RelativeEq for NavVec3 {
    fn default_max_relative() -> Self::Epsilon {
        Scalar::default_max_relative()
    }

    fn relative_eq(
        &self,
        other: &Self,
        epsilon: Self::Epsilon,
        max_relative: Self::Epsilon,
    ) -> bool {
        Scalar::relative_eq(&self.x, &other.x, epsilon, max_relative)
            && Scalar::relative_eq(&self.y, &other.y, epsilon, max_relative)
            && Scalar::relative_eq(&self.z, &other.z, epsilon, max_relative)
    }
}

impl UlpsEq for NavVec3 {
    fn default_max_ulps() -> u32 {
        Scalar::default_max_ulps()
    }

    fn ulps_eq(&self, other: &Self, epsilon: Self::Epsilon, max_ulps: u32) -> bool {
        Scalar::ulps_eq(&self.x, &other.x, epsilon, max_ulps)
            && Scalar::ulps_eq(&self.y, &other.y, epsilon, max_ulps)
            && Scalar::ulps_eq(&self.z, &other.z, epsilon, max_ulps)
    }
}
