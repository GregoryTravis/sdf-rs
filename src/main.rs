// TODO remove these
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate nalgebra as na;

use std::convert::TryInto;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::rc::Rc;
use std::time::Instant;

use apng::Encoder;
use apng::Frame;
use apng::PNGImage;
use na::{Point3, Vector3};

const BLACK: Pixel = Pixel { r: 0, g: 0, b: 0, a: 255 };
const BLACKT: Pixel = Pixel { r: 0, g: 0, b: 0, a: 128 };
const WHITET: Pixel = Pixel { r: 255, g: 255, b: 255, a: 128 };
const NONE: Pixel = Pixel { r: 0, g: 0, b: 0, a: 0 };
const REDT: Pixel = Pixel { r: 255, g: 0, b: 0, a: 128 };

pub fn length(a: f32, b: f32) -> f32 {
  (a*a + b*b).sqrt()
}

#[derive(Clone, Copy, Debug)]
pub struct Pixel {
  pub r: u8,
  pub g: u8,
  pub b: u8,
  pub a: u8,
}

impl Pixel {
  pub fn mix(&self, op: Pixel) -> Pixel{
    Pixel {
      r: avg(self.r, op.r),
      g: avg(self.g, op.g),
      b: avg(self.b, op.b),
      a: avg(self.a, op.a),
    }
  }

  pub fn lerp(&self, p: Pixel, a: f32) -> Pixel {
    Pixel {
      r: ((self.r as f32 * (1.0 - a)) + (p.r as f32 * a)) as u8,
      g: ((self.g as f32 * (1.0 - a)) + (p.g as f32 * a)) as u8,
      b: ((self.b as f32 * (1.0 - a)) + (p.b as f32 * a)) as u8,
      a: ((self.a as f32 * (1.0 - a)) + (p.a as f32 * a)) as u8,
    }
  }
}

fn avgf32(a: f32, b: f32) -> f32 {
  (a + b) / 2.0
}

fn avg(a: u8, b: u8) -> u8 {
  ((a as f32 + b as f32) / 2.0) as u8
}

// OVER Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 0 }
// OVER Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 0 }
// OVER Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 0 }
// OVER Pixel { r: 255, g: 0, b: 0, a: 128 } Pixel { r: 0, g: 0, b: 0, a: 255 } Pixel { r: 255, g: 0, b: 0, a: 0 }
// OVER Pixel { r: 255, g: 0, b: 0, a: 255 } Pixel { r: 0, g: 0, b: 0, a: 128 } Pixel { r: 255, g: 0, b: 0, a: 255 }
// a over b
pub fn over(a: &Pixel, b: &Pixel) -> Pixel {
  let ao = (a.a as f32 + (b.a as f32 * (255.0 - a.a as f32))).clamp(0.0, 255.0);
  let over = Pixel {
    a: ao as u8,
    r: (((a.r as f32 * a.a as f32) + (b.r as f32 * b.a as f32 * (255.0 - a.a as f32))) / ao).clamp(0.0, 255.0)  as u8,
    g: (((a.g as f32 * a.a as f32) + (b.g as f32 * b.a as f32 * (255.0 - a.a as f32))) / ao).clamp(0.0, 255.0)  as u8,
    b: (((a.b as f32 * a.a as f32) + (b.b as f32 * b.a as f32 * (255.0 - a.a as f32))) / ao).clamp(0.0, 255.0)  as u8,
  };
  // println!("OVER {:?} {:?} {:?}", a, b, over);
  over
}

#[derive(Clone, Copy)]
pub struct Pt<T> {
  pub x: T,
  pub y: T,
}

#[derive(Clone, Copy)]
pub struct Rect<T> {
  pub ll: Pt<T>,
  pub ur: Pt<T>,
}

#[allow(unused_variables, dead_code)]
#[derive(Debug)]
pub struct FB {
  pub w: usize,
  pub h: usize,
  pixels: Vec<u8>,
}

impl FB {
  pub fn new(w: usize, h: usize) -> FB {
    let fb = FB {
      w: w,
      h: h,
      pixels: vec![0; w*h*4],
    };
    return fb;
  }

  pub fn get(&self, x: usize, y: usize) -> Pixel {
    let off = ((y*self.w) + x) * 4;
    Pixel { r: self.pixels[off+0], b: self.pixels[off+1], g: self.pixels[off+2], a: self.pixels[off+3] }
  }

  pub fn set(&mut self, x: usize, y: usize, pix: &Pixel) {
    let off = ((y*self.w) + x) * 4;
    // TODO unnecessary copy
    let current = Pixel { r: self.pixels[off+0], b: self.pixels[off+1], g: self.pixels[off+2], a: self.pixels[off+3] };
    let over = over(&pix, &current);
    self.pixels[off+0] = over.r;
    self.pixels[off+1] = over.g;
    self.pixels[off+2] = over.b;
    self.pixels[off+3] = over.a;
  }

  pub fn write(&self, path: String) {
    // let path = Path::new(r"image.png");
    let file = File::create(path).unwrap();
    let ref mut w = BufWriter::new(file);

    let mut encoder = png::Encoder::new(w, self.w.try_into().unwrap(), self.h.try_into().unwrap());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    // encoder.set_trns(vec!(0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8));
    encoder.set_source_gamma(png::ScaledFloat::from_scaled(45455)); // 1.0 / 2.2, scaled by 100000
    encoder.set_source_gamma(png::ScaledFloat::new(1.0 / 2.2));     // 1.0 / 2.2, unscaled, but rounded
    let source_chromaticities = png::SourceChromaticities::new(     // Using unscaled instantiation here
        (0.31270, 0.32900),
        (0.64000, 0.33000),
        (0.30000, 0.60000),
        (0.15000, 0.06000)
    );
    encoder.set_source_chromaticities(source_chromaticities);
    let mut writer = encoder.write_header().unwrap();

    // let arr = Rc::new([u8; self.w * self.h * 4]);
    let sl: &[u8] = &self.pixels;
  // let a = vec![1, 2, 3, 4, 5];
  //   let b: &[i32] = &a;
    writer.write_image_data(sl).unwrap(); // Save

    // let data = self.pixels.try_into()
    //     .unwrap_or_else(|v: Vec<u8>| panic!("oops"));

    // writer.write_image_data(&data).unwrap(); // Save
  }

  // pub fn get_raw<const N: usize>(&self) -> [u8; N] {
  //   // return self.pixels.try_into().unwrap();
  //   self.pixels.try_into()
  //       .unwrap_or_else(|v: Vec<u8>| panic!("Expected a Vec of length {} but it was {}", N, v.len()))
  // }
}

// Doesn't check that dims match
fn downsample_halve(fb: &FB, ofb: &mut FB) {
  // let mut ofb = FB::new(fb.w/2, fb.h/2);
  for x in 0..ofb.w {
    for y in 0..ofb.h {
      let ox = x * 2;
      let oy = y * 2;
      let combined = fb.get(ox, oy).mix(fb.get(ox+1,oy)).mix(fb.get(ox, oy+1).mix(fb.get(ox+1, oy+1)));
      ofb.set(x, y, &combined)
    }
  }
}

fn upsample_render<S>(shape: &S,
                      colorer: fn(shape: &S, x: f32, y:f32) -> Pixel, domain: Rect<f32>, fb: &mut FB)
where
  S: Shape
{
  let mut ufb = FB::new(fb.w*2, fb.h*2);
  render(shape, colorer, domain, &mut ufb);
  downsample_halve(&ufb, fb);
}

fn render<S>(shape: &S, colorer: fn(shape: &S, x: f32, y:f32) -> Pixel, domain: Rect<f32>, fb: &mut FB)
where
  S: Shape
{
  let ox = domain.ll.x;
  let oy = domain.ll.y;
  let dx = (domain.ur.x - domain.ll.x) / (fb.w as f32);
  let dy = (domain.ur.y - domain.ll.y) / (fb.h as f32);
  for x in 0..fb.w {
    for y in 0..fb.h {
      let fx = ox + ((x as f32) * dx);
      let fy = oy + ((y as f32) * dy);
      fb.set(x, y, &colorer(shape, fx, fy));
    }
  }
}

pub trait Shape {
  fn dist(&self, x: f32, y: f32) -> f32;
}

// TODO center, radius?
#[derive(Clone, Copy, Debug)]
pub struct Circle {
}

impl Shape for Circle {
  fn dist(&self, x: f32, y: f32) -> f32 {
    let d = length(x, y) - 1.0;
    // println!("dist {} {} {}|", x, y, d);
    d
  }
}

#[derive(Clone, Copy, Debug)]
pub struct Square {
}

impl Shape for Square {
  fn dist(&self, x: f32, y: f32) -> f32 {
    // -((x.abs() - 1.0).max(y.abs() - 1.0))
    (x.abs() - 1.0).max(y.abs() - 1.0)
  }
}

pub struct Translate {
  shape: Rc<dyn Shape>,
  tx: f32,
  ty: f32,
}

impl Translate {
  pub fn new(shape: Rc<dyn Shape>, tx: f32, ty: f32) -> Translate {
    Translate { shape: shape, tx: tx, ty: ty }
  }
}

impl Shape for Translate {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape.dist(x - self.tx, y - self.ty)
  }
}

pub struct Scale {
  shape: Rc<dyn Shape>,
  sx: f32,
  sy: f32,
}

impl Scale {
  pub fn new(shape: Rc<dyn Shape>, sx: f32, sy: f32) -> Scale {
    Scale { shape: shape, sx: sx, sy: sy }
  }
}

impl Shape for Scale {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape.dist(x / self.sx, y / self.sy)
  }
}

pub struct Union {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl Union {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> Union {
    Union { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Union {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape0.dist(x, y).min(self.shape1.dist(x, y))
  }
}

pub struct Intersection {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl Intersection {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> Intersection {
    Intersection { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Intersection {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape0.dist(x, y).max(self.shape1.dist(x, y))
  }
}

pub struct Difference {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl Difference {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> Difference {
    Difference { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Difference {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape0.dist(x, y).max(-self.shape1.dist(x, y))
  }
}

pub struct Blend {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl Blend {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> Blend {
    Blend { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Blend {
  fn dist(&self, x: f32, y: f32) -> f32 {
    avgf32(self.shape0.dist(x, y), self.shape1.dist(x, y))
  }
}

pub struct SmoothUnion {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl SmoothUnion {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> SmoothUnion {
    SmoothUnion { shape0: shape0, shape1: shape1 }
  }
}

// float2 intersectionSpace = float2(shape1 - radius, shape2 - radius);
// intersectionSpace = min(intersectionSpace, 0);
// float insideDistance = -length(intersectionSpace);
// float simpleUnion = merge(shape1, shape2);
// float outsideDistance = max(simpleUnion, radius);
// return  insideDistance + outsideDistance;
// merge is min

impl Shape for SmoothUnion {
  fn dist(&self, x: f32, y: f32) -> f32 {
    let r = 0.3;
    let d0 = self.shape0.dist(x, y);
    let d1 = self.shape1.dist(x, y);
    let md0 = (d0 - r).min(0.0);
    let md1 = (d1 - r).min(0.0);
    let inside_distance = -length(md0, md1);
    let simple_union = d0.min(d1);
    let outside_distance = simple_union.max(r);
    inside_distance + outside_distance
  }
}

// float round_merge(float shape1, float shape2, float radius){
//     float2 intersectionSpace = float2(shape1, shape2);
//     intersectionSpace = min(intersectionSpace, 0);
//     return length(intersectionSpace);
// }

pub struct Hmm {
  shape0: Rc<dyn Shape>,
  shape1: Rc<dyn Shape>,
}

impl Hmm {
  pub fn new(shape0: Rc<dyn Shape>, shape1: Rc<dyn Shape>) -> Hmm {
    Hmm { shape0: shape0, shape1: shape1 }
  }
}

impl Shape for Hmm {
  fn dist(&self, x: f32, y: f32) -> f32 {
    self.shape0.dist(x, y) + self.shape1.dist(x, y)
  }
}

pub struct Grid {
  w: f32,
  h: f32,
  shape: Rc<dyn Shape>,
}

impl Grid {
  pub fn new(w: f32, h: f32, shape: Rc<dyn Shape>) -> Grid {
    Grid { w: w, h: h, shape: shape }
  }
}

fn grid_fmod(a: f32, b: f32) -> f32 {
  let aob = a / b;
  (aob - aob.floor()) * b
}

impl Shape for Grid {
  fn dist(&self, x: f32, y: f32) -> f32 {
    // let ix = (x / self.w).floor();
    // let iy = (y / self.h).floor();
    // println!("FMOD {} {} {}", 0.5 % 0.3, 12.0 % 5.0, -0.5 % 0.3);
    // println!("FLOOR {} {}", (1.6_f32).floor(), (-1.6_f32).floor());
    // println!("FMOD {} {} {} {} {} {}", grid_fmod(0.3, 0.3), grid_fmod(0.45, 0.3), grid_fmod(0.6, 0.3), grid_fmod(0.5, 0.3), grid_fmod(12.0, 5.0), grid_fmod(-0.5, 0.3));
    // let xx = x % self.w;
    // let yy = y % self.h;
    let xx = grid_fmod(x, self.w);
    let yy = grid_fmod(y, self.h);
    self.shape.dist(xx, yy)
  }
}

// Turn a distance into a color
fn _solid(d: f32) -> Pixel {
  let inside = d > 0.0;
  if inside { REDT } else { BLACK }
}

// Turn a distance into a color
const BANDWIDTH: f32 = 0.1;
fn band(d: f32) -> Pixel {
  let inside = d.abs() < (BANDWIDTH/2.0);
  if inside { REDT } else { NONE }
}

// c: point to color
// ac, bc: tangent vectors
// normal is ac x bc
// a <- c
//     /
//    /
//   L
// b
fn bump_map<S>(shape: &S, cx: f32, cy:f32) -> Pixel
where
  S: Shape
{
  let cdist = shape.dist(cx, cy);

  if cdist > 0.0 {
    return NONE;
  }

  let pi =  std::f32::consts::PI;
  let bit = 0.1;
  let ax = cx - bit;
  let ay = cy;
  let bx = cx - bit;
  let by = cy - bit;

  let adist = shape.dist(ax, ay);
  let bdist = shape.dist(bx, by);

  let az = ((pi/2.0) * (adist+1.0)).cos();
  let bz = ((pi/2.0) * (bdist+1.0)).cos();
  let cz = ((pi/2.0) * (cdist+1.0)).cos();

  let a = Point3::new(ax, ay, az);
  let b = Point3::new(bx, by, bz);
  let c = Point3::new(cx, cy, cz);

  let ac: Vector3<f32> = a - c;
  let bc: Vector3<f32> = b - c;

  let norm = ac.cross(&bc).normalize();

  let light = Vector3::new(-1.0, 1.0, 1.0).normalize();

  let brightness = norm.dot(&light);

  // println!("BUMP");
  // println!("cx cy {} {}", cx, cy);
  // println!("dists {} {} {}", adist, bdist, cdist);
  // println!("{}", a);
  // println!("{}", b);
  // println!("{}", c);
  // println!("{}", ac);
  // println!("{}", bc);
  // println!("{}", ac.cross(&bc));
  // println!("norm {} light {} brightness {}", norm, light, brightness);

  BLACKT.lerp(WHITET, brightness.clamp(0.0, 1.0))
}

// TODO slow
// Ruler
const RULE_SEP: f32 = 4.0;
const RULE_WIDTH: f32 = 0.05;
const SUB_RULE_WIDTH: f32 = 0.1;
const SUB_RULE_COUNT: f32 = 4.0;
fn ruler<S>(shape: &S, x: f32, y:f32) -> Pixel
where
  S: Shape
{
  let d = shape.dist(x, y);
  let dist_from_unit = ((d*RULE_SEP) - (d*RULE_SEP).floor()).abs() / RULE_SEP;
  let rule = dist_from_unit < RULE_WIDTH;
  // let dist_from_sub_unit = ((d*SUB_RULE_COUNT) - ((d*SUB_RULE_COUNT).floor())).abs();
  // let sub_rule = dist_from_sub_unit < SUB_RULE_WIDTH;
  let inside = d < 0.0;
  if inside {
    REDT
  // } else if rule || sub_rule {
  } else if rule {
    BLACKT
  } else {
    NONE
  }
}

// fn compose<A, B, C, G, F>(f: F, g: G) -> impl Fn(A) -> C
// where
//     F: Fn(A) -> B,
//     G: Fn(B) -> C,
// {
//     move |x| g(f(x))
// }

fn main() {
    println!("Hello, world!");
  // let path = Path::new(r"image.png");
  // let file = File::create(path).unwrap();
  // let ref mut w = BufWriter::new(file);

  // let mut encoder = png::Encoder::new(w, 2, 2); // Width is 2 pixels and height is 1.
  // encoder.set_color(png::ColorType::Rgba);
  // encoder.set_depth(png::BitDepth::Eight);
  // encoder.set_trns(vec!(0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8));
  // encoder.set_source_gamma(png::ScaledFloat::from_scaled(45455)); // 1.0 / 2.2, scaled by 100000

  // encoder.set_source_gamma(png::ScaledFloat::new(1.0 / 2.2));     // 1.0 / 2.2, unscaled, but rounded
  // let source_chromaticities = png::SourceChromaticities::new(     // Using unscaled instantiation here
  //     (0.31270, 0.32900),
  //     (0.64000, 0.33000),
  //     (0.30000, 0.60000),
  //     (0.15000, 0.06000)
  // );
  // encoder.set_source_chromaticities(source_chromaticities);
  // let mut writer = encoder.write_header().unwrap();

  // let data = [255, 0, 0, 255, 0, 0, 0, 255,
  //             0, 255, 0, 255, 0, 0, 255, 255]; // An array containing a RGBA sequence. First pixel is red and second pixel is black.
  // let data = fb.get_raw::<N>();

  // let mut fb: FB = FB::new(2, 2);
  // let red = Pixel { r: 255, g: 0, b: 0, a: 255 };
  // let green = Pixel { r: 0, g: 255, b: 0, a: 255 };
  // let blue = Pixel { r: 0, g: 0, b: 255, a: 255 };
  // let white = Pixel { r: 255, g: 255, b: 255, a: 255 };
  // fb.set(0, 0, &green);
  // fb.set(0, 1, &red);
  // fb.set(1, 0, &blue);
  // fb.set(1, 1, &white);
  // // fb.write("image.png".to_string());

  let (w, h) = (800, 800);
  // let (w, h) = (20, 20);
  // let (w, h) = (2, 2);
  // let (w, h) = (4, 4);
  // let mut cfb = FB::new(w, h);
  let vd = 8.0;
  let view = Rect { ll: Pt { x: -vd, y: -vd }, ur: Pt { x: vd, y: vd } };
  let circle = Circle {};
  let moved_half = Translate::new(Rc::new(circle), 0.5, 0.5);
  let ucircle = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(circle), 0.5, 0.5)), 1.0, 1.0));
  // let moved = Translate::new(Rc::new(circle), 1.0, 0.0);
  // let moved2 = Translate::new(Rc::new(circle), 1.0, 0.0);
  // let moved3 = Translate::new(Rc::new(circle), 1.0, 0.0);
  // let moved4 = Translate::new(Rc::new(circle), 1.0, 0.0);
  // let moved5 = Translate::new(Rc::new(circle), 1.0, 0.0);
  // let inter = Intersection::new(Rc::new(circle), Rc::new(moved));
  // let union = Union::new(Rc::new(circle), Rc::new(moved2));
  // let diff = Difference::new(Rc::new(circle), Rc::new(moved4));
  // let hmm = Hmm::new(Rc::new(circle), Rc::new(moved3));
  // let smooth = SmoothUnion::new(Rc::new(circle), Rc::new(moved5));
  let grid = Rc::new(Grid::new(2.0, 2.0, ucircle.clone()));
  let grid2 = Rc::new(Scale::new(grid.clone(), 2.5, 2.5));
  let gridi = Intersection::new(grid.clone(), grid2.clone());
  let gridu = Translate::new(Rc::new(Union::new(grid.clone(), grid2.clone())), 0.2, 0.2);

  // // let start = Instant::now();
  // render(&gridi, ruler, view, &mut cfb);
  // // eprintln!("elapsed {:?}", start.elapsed()); // note :?
  // render(&gridu, ruler, view, &mut cfb);

  // render(&inter, band, Rect { ll: Pt { x: -2.0, y: -2.0 }, ur: Pt { x: 2.0, y: 2.0 } }, &mut cfb);
  // render(&union, band, Rect { ll: Pt { x: -2.0, y: -2.0 }, ur: Pt { x: 2.0, y: 2.0 } }, &mut cfb);
  // render(&hmm, band, Rect { ll: Pt { x: -2.0, y: -2.0 }, ur: Pt { x: 2.0, y: 2.0 } }, &mut cfb);
  // render(&diff, band, Rect { ll: Pt { x: -2.0, y: -2.0 }, ur: Pt { x: 2.0, y: 2.0 } }, &mut cfb);
  // render(&smooth, band, Rect { ll: Pt { x: -2.0, y: -2.0 }, ur: Pt { x: 2.0, y: 2.0 } }, &mut cfb);

    // cfb.write("image.png".to_string());
    let mut files = Vec::new();
    let num_frames = 1000;
    for x in 0..num_frames {
      let mut acfb = FB::new(w, h);
      let filename = format!("image{:0>10}.png", x);
      let dt = (x as f32) / 40.0;
      // let (i, u) = wacky(dt);
      // render(&i, ruler, view, &mut acfb);
      // render(&u, ruler, view, &mut acfb);
      let s = wacky2(dt);
      // let s = cgrid_circle(dt);
      let start = Instant::now();
      // upsample_render(&s, ruler, view, &mut acfb);
      upsample_render(&s, bump_map, view, &mut acfb);
      eprintln!("elapsed {:?}", start.elapsed()); // note :?
      // eprintln!("fb {:?}", acfb);
      acfb.write(filename.clone());
      files.push(filename);
    }

    let mut png_images: Vec<PNGImage> = Vec::new();
    for f in files.iter() {
        png_images.push(apng::load_png(f).unwrap());
    }

    let path = Path::new(r"anim.png");
    let mut out = BufWriter::new(File::create(path).unwrap());

    let config = apng::create_config(&png_images, None).unwrap();
    let mut encoder = Encoder::new(&mut out, config).unwrap();
    let frame = Frame {
        delay_num: Some(1),
        delay_den: Some(20),
        ..Default::default()
    };

    match encoder.encode_all(png_images, Some(&frame)) {
        Ok(_n) => println!("success"),
        Err(err) => eprintln!("{}", err),
    }

    let clean_up = true;
    if clean_up {
        for filename in files {
            match std::fs::remove_file(filename) {
                Ok(_n) => (),
                Err(err) => eprintln!("{}", err),
            }
        }
    }
}

fn just_circle(_t: f32) -> impl Shape {
  Circle {}
}

fn cgrid_circle(_t: f32) -> impl Shape {
  let sc = Circle {};
  let ucircle = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(sc), 0.5, 0.5)), 1.0, 1.0));
  let grid = Grid::new(2.0, 2.0, ucircle.clone());
  grid
}

fn blah(_t: f32) -> impl Shape {
  let s = Square{};
  let c = Circle{};
  Blend::new(Rc::new(c), Rc::new(s))
}

fn blend(_t: f32) -> impl Shape {
  let c = Rc::new(Circle {});
  let s = Rc::new(Square {});
  let c0 = Translate::new(c.clone(), -0.90, 0.0);
  let c1 = Translate::new(s.clone(),  0.75, 0.0);
  let all = SmoothUnion::new(Rc::new(c0), Rc::new(c1));
  // let all = Union::new(Rc::new(c0), Rc::new(c1));
  all
}

fn wacky2(t: f32) -> impl Shape {
  // let circle = Circle {};
  let s = Square{};
  let c = Circle{};
  let sc = Blend::new(Rc::new(c), Rc::new(s));

  let ucircle = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(sc), 0.5, 0.5)), 1.0, 1.0));
  let grid = Rc::new(Grid::new(2.0, 2.0, ucircle.clone()));
  let grid2 = Rc::new(Scale::new(grid.clone(), 2.5, 2.5));
  let gridi = SmoothUnion::new(Rc::new(Translate::new(grid.clone(), 0.2 + t, 0.2)), Rc::new(Translate::new(grid2.clone(), 0.2, 0.2 + t)));
  gridi
}

fn wacky(t: f32) -> (impl Shape, impl Shape) {
  let circle = Circle {};
  let ucircle = Rc::new(Translate::new(Rc::new(Scale::new(Rc::new(circle), 0.5, 0.5)), 1.0, 1.0));
  let grid = Rc::new(Grid::new(2.0, 2.0, ucircle.clone()));
  let grid2 = Rc::new(Scale::new(grid.clone(), 2.5, 2.5));
  let gridi = Intersection::new(grid.clone(), grid2.clone());
  let gridu = Translate::new(Rc::new(Union::new(grid.clone(), grid2.clone())), 0.2, 0.2 + t);
  (gridi, gridu)
}
