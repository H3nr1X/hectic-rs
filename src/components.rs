use cgmath::Vector2;
use specs::*;
use cgmath::MetricSpace;

use crate::{WIDTH, HEIGHT};
use crate::graphics::Image as GraphicsImage;

#[derive(Component)]
pub struct Image(u16);

impl Image {
    pub fn coordinates(&self) -> (f32, f32, f32, f32) {
        let (x, y, w, h) = GraphicsImage::from_u16(self.0).coordinates();
        let size = GraphicsImage::from_u16(self.0).image_size() as f32;
        (x as f32 / size, y as f32 / size, w as f32 / size, h as f32 / size)
    }

    pub fn size(&self) -> (f32, f32) {
        let (_, _, w, h) = GraphicsImage::from_u16(self.0).coordinates();
        (w as f32, h as f32)
    }

    pub fn from(image: GraphicsImage) -> Self {
        Self(image.to_u16())
    }
}

#[derive(Component)]
pub struct Controllable;

#[derive(Component)]
pub struct BackgroundLayer;

#[derive(Component)]
pub struct Position(pub Vector2<f32>);

#[derive(Component, Clone)]
pub enum Movement {
    Linear(Vector2<f32>),
    Falling(f32),
    FollowCurve(Curve)
}

#[derive(Component)]
pub struct DieOffscreen;

#[derive(Component)]
pub struct BeenOnscreen;

#[derive(Component)]
pub struct FrozenUntil(pub f32);

const S: f32 = 0.0;

const CURVE_BASIS_MATRIX: [[f32; 4]; 4] = [
    [(S-1.0)/2.0, (S+3.0)/2.0,  (-3.0-S)/2.0, (1.-S)/2.0],
    [(1.-S), (-5.-S)/2., (S+2.), (S-1.)/2.],
    [(S-1.)/2., 0., (1.-S)/2., 0.],
    [0., 1., 0., 0.]
];

fn curve_point_scalar(a: f32, b: f32, c: f32, d: f32, t: f32) -> f32 {
    let tt = t * t;
    let ttt = t * tt;
    let cb = CURVE_BASIS_MATRIX;

    return  a * (ttt*cb[0][0] + tt*cb[1][0] + t*cb[2][0] + cb[3][0]) +
            b * (ttt*cb[0][1] + tt*cb[1][1] + t*cb[2][1] + cb[3][1]) +
            c * (ttt*cb[0][2] + tt*cb[1][2] + t*cb[2][2] + cb[3][2]) +
            d * (ttt*cb[0][3] + tt*cb[1][3] + t*cb[2][3] + cb[3][3]);
}

#[derive(Clone)]
pub struct Curve {
    pub a: Vector2<f32>,
    pub b: Vector2<f32>,
    pub c: Vector2<f32>,
    pub d: Vector2<f32>,
    pub time: f32,
    pub speed: f32,
}

impl Curve {
    fn point(&self, time: f32) -> Vector2<f32> {
        Vector2::new(
            curve_point_scalar(self.a.x, self.b.x, self.c.x, self.d.x, time),
            curve_point_scalar(self.a.y, self.b.y, self.c.y, self.d.y, time)
        )
    }

    pub fn step(&mut self, previous_point: Vector2<f32>) -> Vector2<f32> {
        let mut min_time = self.time;
        let mut max_time = self.time + 1.0;

        loop {
            let mid_time = (min_time + max_time) / 2.0;
            let mid_point = self.point(mid_time);
            let mid_dist = mid_point.distance(previous_point);

            // If it's precise enough, set it and return
            if (mid_dist - self.speed).abs() < 0.1 {
                self.time = mid_time;
                return mid_point;
            // Else change the min/max values
            } else if mid_dist < self.speed {
                min_time = mid_time;
            } else {
                max_time = mid_time;
            }
        }
    }


    pub fn horizontal(start_y: f32, end_y: f32, left_to_right: bool, speed: f32) -> Self {
        const FORCE: f32 = 1500.0;
        const OFFSET: f32 = 20.0;

        if left_to_right {
            Self {
                a: Vector2::new(-FORCE - OFFSET, start_y),
                b: Vector2::new(-OFFSET, start_y),
                c: Vector2::new(WIDTH + OFFSET, end_y),
                d: Vector2::new(WIDTH + FORCE + OFFSET, end_y),
                time: 0.0,
                speed,
            }
        } else {
            Self {
                a: Vector2::new(WIDTH + FORCE + OFFSET, start_y),
                b: Vector2::new(WIDTH + OFFSET, start_y),
                c: Vector2::new(-OFFSET, end_y),
                d: Vector2::new(-FORCE - OFFSET, end_y),
                time: 0.0,
                speed,
            }
        }
    }

    pub fn vertical(mut start_x: f32, mut end_x: f32, speed: f32) -> Self {
        start_x *= WIDTH;
        end_x *= WIDTH;
        let force = 2000.0;

        Self {
            a: Vector2::new(start_x, -20.0 -force),
            b: Vector2::new(start_x, -20.0),
            c: Vector2::new(end_x, HEIGHT),
            d: Vector2::new(end_x, HEIGHT + force),
            time: 0.0,
            speed,
        }
    }
}
